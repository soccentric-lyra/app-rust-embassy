//! Debug log lines on USART6 (PG14, Arduino D1, 115200 8N1, TX only).
//!
//! USART3 carries HWLINK only.  Lines use the format shared with BOOT:
//!
//! ```text
//! [   5012] R-EMB I health: passed, CONFIRMED written
//! ```
//!
//! RDBG-002: the macros format into a stack buffer and push the line into a
//! 1 KB pipe without waiting; a full pipe drops the line and counts it.  The
//! log task drains the pipe with DMA.  RDBG-004: `flush_blocking` drains the
//! rest by polling before a deliberate reset.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RDBG-001..RDBG-004

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};

use embassy_stm32::mode::Async;
use embassy_stm32::usart::UartTx;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_time::Instant;

/// Firmware tag printed on every line.
const TAG: &str = "R-EMB";
/// Maximum line length in bytes, including CR LF; longer text is cut.
const LINE_MAX: usize = 128;

/// Queue of formatted lines waiting for the log task.
static PIPE: Pipe<CriticalSectionRawMutex, 1024> = Pipe::new();
/// Lines dropped because [`PIPE`] was full.
static DROPPED: AtomicU32 = AtomicU32::new(0);

/// Stack buffer one line is formatted into.
struct Line {
    /// Line bytes.
    buf: [u8; LINE_MAX],
    /// Bytes used in `buf`.
    len: usize,
}

impl Write for Line {
    /// Append `s`, truncating silently; never fails.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Keep two bytes for CR LF.
        let take = s.len().min(LINE_MAX - 2 - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

/// Queue one line at `level` (`'E'`, `'W'` or `'I'`) without blocking.
///
/// Drops the whole line and counts it when the pipe lacks room.  Call from
/// executor context only.
pub fn write(level: char, args: fmt::Arguments) {
    let mut line = Line {
        buf: [0; LINE_MAX],
        len: 0,
    };
    let ms = Instant::now().as_millis() as u32;
    let _ = write!(line, "[{:7}] {} {} ", ms, TAG, level);
    let _ = line.write_fmt(args);
    line.buf[line.len] = b'\r';
    line.buf[line.len + 1] = b'\n';
    let bytes = &line.buf[..line.len + 2];
    // All writers run on the one executor thread, so nothing can fill the
    // pipe between the check and the write.
    if PIPE.free_capacity() < bytes.len() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    // try_write() stops at the end of the ring, so a line that straddles the
    // wrap point needs a second call for its tail.  The capacity was checked
    // above, so this always finishes.
    let mut rest = bytes;
    while !rest.is_empty() {
        match PIPE.try_write(rest) {
            Ok(n) => rest = &rest[n..],
            Err(_) => {
                DROPPED.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }
}

/// Rate-limited `task <n> <name>: <purpose>` line, at most once a second.
///
/// Each task owns one and calls [`TaskReport::tick`] once per loop.
pub struct TaskReport {
    /// Task number printed in the line.
    number: u8,
    /// Task name.
    name: &'static str,
    /// One-line task description.
    purpose: &'static str,
    /// Time of the last printed line; `None` before the first.
    last: Option<Instant>,
}

impl TaskReport {
    /// Create a report that prints on its first `tick`.
    pub const fn new(number: u8, name: &'static str, purpose: &'static str) -> Self {
        Self {
            number,
            name,
            purpose,
            last: None,
        }
    }

    /// Print the line if at least 1000 ms passed since the last one.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let due = match self.last {
            Some(last) => now.duration_since(last).as_millis() >= 1000,
            None => true,
        };
        if due {
            self.last = Some(now);
            write(
                'I',
                format_args!("task {} {}: {}", self.number, self.name, self.purpose),
            );
        }
    }
}

/// Lines dropped because the pipe was full (RDBG-003).
#[allow(dead_code)]
pub fn dropped() -> u32 {
    DROPPED.load(Ordering::Relaxed)
}

/// Log task: drain [`PIPE`] to USART6 by DMA in chunks of up to 64 bytes.
#[embassy_executor::task]
pub async fn task(mut tx: UartTx<'static, Async>) {
    let mut chunk = [0u8; 64];
    loop {
        let n = PIPE.read(&mut chunk).await;
        let _ = tx.write(&chunk[..n]).await;
    }
}

/// USART6 interrupt and status register (RM0410 section 34.8).
const USART6_ISR: *const u32 = 0x4001_141C as *const u32;
/// USART6 transmit data register.
const USART6_TDR: *mut u32 = 0x4001_1428 as *mut u32;
/// ISR transmission complete flag.
const ISR_TC: u32 = 1 << 6;
/// ISR transmit data register empty flag.
const ISR_TXE: u32 = 1 << 7;
/// Poll limit for one register wait, so a stuck USART cannot hang a reset.
const WAIT_LOOPS: u32 = 200_000;

/// Poll USART6_ISR until `flag` is set or [`WAIT_LOOPS`] polls pass.
fn wait_flag(flag: u32) {
    for _ in 0..WAIT_LOOPS {
        // SAFETY: fixed, always-mapped USART6 status register; read only.
        if unsafe { USART6_ISR.read_volatile() } & flag != 0 {
            return;
        }
    }
}

/// Write out the queued lines by polling USART6 (RDBG-004).
///
/// Call just before a reset, when the log task no longer runs.  Waits for TC
/// first so a DMA chunk in flight can finish.
pub fn flush_blocking() {
    wait_flag(ISR_TC);
    let mut chunk = [0u8; 64];
    while let Ok(n) = PIPE.try_read(&mut chunk) {
        for &b in &chunk[..n] {
            wait_flag(ISR_TXE);
            // SAFETY: fixed USART6 data register; the log task is not
            // running (we are about to reset).
            unsafe { USART6_TDR.write_volatile(u32::from(b)) };
        }
    }
    wait_flag(ISR_TC);
}

/// Log an error line (`E`); takes `format_args!` arguments.
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::system::log::write('E', format_args!($($arg)*)) };
}

/// Log a warning line (`W`); takes `format_args!` arguments.
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::system::log::write('W', format_args!($($arg)*)) };
}

/// Log an info line (`I`); takes `format_args!` arguments.
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::system::log::write('I', format_args!($($arg)*)) };
}
