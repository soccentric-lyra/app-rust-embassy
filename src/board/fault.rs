//! Panic and fault handlers, the no-init fault record and stack painting.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RGEN-004, RGEN-005, RRT-005; DSAFE / LED-005 fault indication

use core::arch::naked_asm;
use core::panic::PanicInfo;
use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};
use core::sync::atomic::{AtomicUsize, Ordering};

use cortex_m::asm;
use cortex_m_rt::{exception, ExceptionFrame};

use crate::board;

/// Valid-record magic, "FAUL".
pub const MAGIC: u32 = 0x4641_554C;

/// [`FaultRecord::kind`]: Rust panic.
pub const PANIC: u32 = 1;
/// [`FaultRecord::kind`]: HardFault.
pub const HARD_FAULT: u32 = 2;
/// [`FaultRecord::kind`]: MemManage fault.
pub const MEM_MANAGE: u32 = 3;
/// [`FaultRecord::kind`]: BusFault.
pub const BUS_FAULT: u32 = 4;
/// [`FaultRecord::kind`]: UsageFault.
pub const USAGE_FAULT: u32 = 5;

/// Panic file name capacity in bytes; longer names are cut.
pub const FILE_NAME_MAX: usize = 40;
/// Panic message capacity in bytes; longer messages are cut.
pub const MESSAGE_MAX: usize = 64;

/// Fault record kept in no-init RAM across a reset.
///
/// The layout is `repr(C)` with `magic` first; [`save`] relies on that.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FaultRecord {
    /// [`MAGIC`] when the record is complete.
    pub magic: u32,
    /// Fault kind, [`PANIC`]..[`USAGE_FAULT`].
    pub kind: u32,
    /// Stacked PC; 0 for a panic.
    pub pc: u32,
    /// Stacked LR; 0 for a panic.
    pub lr: u32,
    /// SCB CFSR at the fault.
    pub cfsr: u32,
    /// SCB HFSR at the fault.
    pub hfsr: u32,
    /// SCB MMFAR at the fault.
    pub mmfar: u32,
    /// SCB BFAR at the fault.
    pub bfar: u32,
    /// Panic source line; 0 for an exception.
    pub line: u32,
    /// Panic source file bytes, not NUL-terminated.
    pub file: [u8; FILE_NAME_MAX],
    /// Bytes used in `file`.
    pub file_len: u32,
    /// Panic message bytes (`panic-messages` feature only).
    pub message: [u8; MESSAGE_MAX],
    /// Bytes used in `message`.
    pub message_len: u32,
}

/// All-zero record; magic 0 marks it invalid.
const EMPTY: FaultRecord = FaultRecord {
    magic: 0,
    kind: 0,
    pc: 0,
    lr: 0,
    cfsr: 0,
    hfsr: 0,
    mmfar: 0,
    bfar: 0,
    line: 0,
    file: [0; FILE_NAME_MAX],
    file_len: 0,
    message: [0; MESSAGE_MAX],
    message_len: 0,
};

/// The fault record in no-init RAM; survives a system reset and is not
/// cleared by cortex-m-rt.
#[link_section = ".noinit.fault"]
static mut RECORD: FaultRecord = EMPTY;

/// Return a copy of the fault record if its magic is valid.
pub fn last() -> Option<FaultRecord> {
    // SAFETY: reads the no-init record by volatile copy; the linker places it
    // in the reserved no-init window and nothing else writes it except the
    // fault paths.
    let record = unsafe { read_volatile(addr_of!(RECORD)) };
    if record.magic == MAGIC {
        Some(record)
    } else {
        None
    }
}

/// Erase the fault record, invalidating the magic first.
///
/// Call from task context only.
pub fn clear() {
    // SAFETY: sole write to the no-init fault record; interrupts are not
    // required because this is called from the comms task.
    unsafe { (addr_of_mut!(RECORD) as *mut u32).write_volatile(0) };
    asm::dmb();
    // SAFETY: same record; EMPTY has magic 0 as its first word.
    unsafe { write_volatile(addr_of_mut!(RECORD), EMPTY) };
    asm::dsb();
}

/// Store `record`, publishing the magic last.
///
/// A reset mid-copy leaves magic 0, so a torn record is never reported (the
/// C demo does the same).
fn save(mut record: FaultRecord) {
    record.magic = 0;
    // SAFETY: sole write to the no-init fault record.  The volatile write is
    // intentional: it must survive the reset that follows.
    unsafe { write_volatile(addr_of_mut!(RECORD), record) };
    asm::dsb();
    // SAFETY: magic is the first word of the repr(C) record; publish it last.
    unsafe { (addr_of_mut!(RECORD) as *mut u32).write_volatile(MAGIC) };
    asm::dsb();
}

/// Read a Cortex-M7 SCB register by address.
fn scb_reg(address: usize) -> u32 {
    // SAFETY: fixed Cortex-M7 System Control Block registers.
    unsafe { (address as *const u32).read_volatile() }
}

/// Fixed buffer the panic message is formatted into.
#[cfg(feature = "panic-messages")]
struct MessageBuffer<'a> {
    /// Destination bytes.
    bytes: &'a mut [u8],
    /// Bytes used in `bytes`.
    len: usize,
}

#[cfg(feature = "panic-messages")]
impl core::fmt::Write for MessageBuffer<'_> {
    /// Append `text`, truncating silently; never fails.
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let n = text.len().min(self.bytes.len().saturating_sub(self.len));
        self.bytes[self.len..self.len + n].copy_from_slice(&text.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

/// Format the panic message; return the bytes and their length.
#[cfg(feature = "panic-messages")]
fn panic_message(info: &PanicInfo<'_>) -> ([u8; MESSAGE_MAX], u32) {
    let mut message = [0u8; MESSAGE_MAX];
    let len = {
        let mut buffer = MessageBuffer {
            bytes: &mut message,
            len: 0,
        };
        let _ = core::fmt::write(&mut buffer, format_args!("{}", info.message()));
        buffer.len
    };
    (message, len as u32)
}

/// Save a [`PANIC`] record with the location, SCB fault registers and, with
/// `panic-messages`, the message.
pub fn record_panic(info: &PanicInfo<'_>) {
    let mut file = [0u8; FILE_NAME_MAX];
    let mut file_len = 0u32;
    let line = match info.location() {
        Some(l) => {
            let bytes = l.file().as_bytes();
            let n = bytes.len().min(FILE_NAME_MAX);
            file[..n].copy_from_slice(&bytes[..n]);
            file_len = n as u32;
            l.line()
        }
        None => 0,
    };
    #[cfg(feature = "panic-messages")]
    let (message, message_len) = panic_message(info);
    #[cfg(not(feature = "panic-messages"))]
    let (message, message_len) = ([0u8; MESSAGE_MAX], 0u32);
    save(FaultRecord {
        magic: MAGIC,
        kind: PANIC,
        pc: 0,
        lr: 0,
        cfsr: scb_reg(0xE000_ED28),
        hfsr: scb_reg(0xE000_ED2C),
        mmfar: scb_reg(0xE000_ED34),
        bfar: scb_reg(0xE000_ED38),
        line,
        file,
        file_len,
        message,
        message_len,
    });
}

/// Save a record of `kind` with PC/LR from the exception frame.
pub fn record_exception(kind: u32, frame: &ExceptionFrame) {
    save(FaultRecord {
        magic: MAGIC,
        kind,
        pc: frame.pc(),
        lr: frame.lr(),
        cfsr: scb_reg(0xE000_ED28),
        hfsr: scb_reg(0xE000_ED2C),
        mmfar: scb_reg(0xE000_ED34),
        bfar: scb_reg(0xE000_ED38),
        line: 0,
        file: [0; FILE_NAME_MAX],
        file_len: 0,
        message: [0; MESSAGE_MAX],
        message_len: 0,
    });
}

/// Save a record of `kind` with the given stacked PC and LR.
fn record_exception_raw(kind: u32, pc: u32, lr: u32) {
    save(FaultRecord {
        magic: MAGIC,
        kind,
        pc,
        lr,
        cfsr: scb_reg(0xE000_ED28),
        hfsr: scb_reg(0xE000_ED2C),
        mmfar: scb_reg(0xE000_ED34),
        bfar: scb_reg(0xE000_ED38),
        line: 0,
        file: [0; FILE_NAME_MAX],
        file_len: 0,
        message: [0; MESSAGE_MAX],
        message_len: 0,
    });
}

/// Return the short name of the record's fault kind.
#[allow(dead_code)]
pub fn describe(record: &FaultRecord) -> &'static str {
    match record.kind {
        PANIC => "panic",
        HARD_FAULT => "hardfault",
        MEM_MANAGE => "memmanage",
        BUS_FAULT => "busfault",
        USAGE_FAULT => "usagefault",
        _ => "fault",
    }
}

// ---------------------------------------------------------------- stack use

/// Fill word for unused stack.
const STACK_PAINT: u32 = 0xC0DE_5A5A;
/// Bytes below the current MSP left unpainted.
const STACK_GUARD: usize = 128;

extern "C" {
    /// Top of the main stack (initial MSP), from cortex-m-rt.
    static _stack_start: u32;
    /// Bottom of the main stack, from cortex-m-rt.
    static _stack_end: u32;
}

/// Paint the unused main stack with [`STACK_PAINT`] (RRT-005).
///
/// Call once at start-up, before the executor runs.  [`STACK_GUARD`] bytes
/// below the current MSP stay unpainted so the loop itself and a late
/// exception are safe.
pub fn paint_stack() {
    let top = addr_of!(_stack_start) as usize;
    let bottom = addr_of!(_stack_end) as usize;
    if bottom >= top {
        return;
    }
    let sp = cortex_m::register::msp::read() as usize;
    let limit = sp.saturating_sub(STACK_GUARD).max(bottom);
    let mut address = (bottom + 3) & !3;
    while address + 4 <= limit {
        // SAFETY: the range is inside the linker-provided stack area and
        // below the current stack pointer, so it is unused at this point.
        unsafe { write_volatile(address as *mut u32, STACK_PAINT) };
        address += 4;
    }
}

/// Address of the lowest non-painted word found so far; 0 before the first
/// scan.
static HWM: AtomicUsize = AtomicUsize::new(0);

/// Return the main stack high-water mark in bytes (RRT-005).
///
/// The first call scans up from the bottom; later calls only scan below the
/// previous frontier, so the 200 ms health poll does not walk the whole stack.
pub fn stack_high_water() -> u32 {
    let top = addr_of!(_stack_start) as usize;
    let bottom = addr_of!(_stack_end) as usize;
    if bottom >= top {
        return 0;
    }

    let frontier = HWM.load(Ordering::Relaxed);
    if frontier == 0 {
        let mut address = (bottom + 3) & !3;
        while address + 4 <= top {
            // SAFETY: reads the linker-provided stack area; the paint pattern
            // was written by paint_stack and any non-painted word is live.
            let word = unsafe { read_volatile(address as *const u32) };
            if word != STACK_PAINT {
                HWM.store(address, Ordering::Relaxed);
                return top.saturating_sub(address) as u32;
            }
            address += 4;
        }
        HWM.store(top, Ordering::Relaxed);
        return 0;
    }

    // Only the region below the previous frontier can have new usage: the
    // stack grows down and the paint is never re-applied.
    let mut address = frontier & !3;
    while address >= bottom + 4 {
        address -= 4;
        // SAFETY: the address is inside the linker-provided stack area.
        let word = unsafe { read_volatile(address as *const u32) };
        if word != STACK_PAINT {
            HWM.store(address, Ordering::Relaxed);
            break;
        }
    }
    top.saturating_sub(HWM.load(Ordering::Relaxed)) as u32
}

// ---------------------------------------------------------------- fault LEDs

/// Configure LD3 (PB14) as an output by register access.
fn led3_init() {
    // SAFETY: fixed RCC/GPIOB registers (RM0410); interrupts are disabled by
    // fatal() so nothing else touches them.
    unsafe {
        board::RCC_AHB1ENR
            .write_volatile(board::RCC_AHB1ENR.read_volatile() | board::RCC_AHB1ENR_GPIOBEN);
        let moder = board::GPIOB_MODER.read_volatile() & !(0b11 << (board::LD3_PIN * 2));
        board::GPIOB_MODER.write_volatile(moder | (0b01 << (board::LD3_PIN * 2)));
    }
}

/// Drive LD3 on or off.
fn led3(on: bool) {
    // SAFETY: GPIOB BSRR write; the pin was configured as an output above.
    unsafe {
        if on {
            board::GPIOB_BSRR.write_volatile(1 << board::LD3_PIN);
        } else {
            board::GPIOB_BSRR.write_volatile(1 << (board::LD3_PIN + 16));
        }
    }
}

/// Flash the `DEMO` Appendix A fault pattern on LD3, then reset
/// (RGEN-004/005).
///
/// Disables interrupts, shows three 100 ms flashes and a 500 ms pause twice
/// (about 2.2 s, busy-wait at 216 MHz) and never returns.
pub fn fatal() -> ! {
    cortex_m::interrupt::disable();
    led3_init();

    for _ in 0..2 {
        for _ in 0..3 {
            led3(true);
            asm::delay(22_000_000); // ~100 ms at 216 MHz
            led3(false);
            asm::delay(22_000_000);
        }
        asm::delay(108_000_000); // ~500 ms pause
    }

    asm::dsb();
    cortex_m::peripheral::SCB::sys_reset()
}

/// Record the panic and reset through [`fatal`].
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    record_panic(info);
    fatal()
}

/// Record a HardFault and reset through [`fatal`].
///
/// # Safety
///
/// Called only by the hardware through the vector table.
// SAFETY: the vector table is the only caller; the frame is the hardware
// exception frame and is valid for the duration of the handler.
#[exception]
unsafe fn HardFault(frame: &ExceptionFrame) -> ! {
    record_exception(HARD_FAULT, frame);
    fatal()
}

// MemManage, BusFault and UsageFault are separate exceptions on the Cortex-M7
// (SHCSR enables them in main).  cortex-m-rt only passes the exception frame
// to HardFault, so the other three use the same naked trampoline as the C
// demo: LR bit 2 selects MSP or PSP, and the stacked PC/LR are r0[6]/r0[5].
/// Record a MemManage/BusFault/UsageFault from its trampoline and reset.
///
/// `frame` is the stacked exception frame; `kind` is a `FaultRecord::kind`.
// SAFETY: the name and ABI match the C declaration in platform/platform.c and
// the naked trampolines below; no Rust code calls it by another signature.
#[unsafe(no_mangle)]
pub extern "C" fn fault_handler_c(frame: *const u32, kind: u32) -> ! {
    // SAFETY: the naked trampolines pass the hardware exception frame, which
    // contains at least the eight words through xPSR (r0..r3, r12, lr, pc).
    let pc = unsafe { read_volatile(frame.add(6)) };
    // SAFETY: same frame; word 5 is the stacked LR.
    let lr = unsafe { read_volatile(frame.add(5)) };
    record_exception_raw(kind, pc, lr);
    fatal()
}

/// Define a naked exception handler `$name`, exported as `$symbol`, that
/// passes the active stack frame and `$kind` to [`fault_handler_c`].
macro_rules! fault_trampoline {
    ($name:ident, $symbol:literal, $kind:expr) => {
        /// Naked fault trampoline; see `fault_trampoline!`.
        ///
        /// # Safety
        ///
        /// Called only by the hardware through the vector table.
        // SAFETY: naked function with no Rust ABI, entered only through the
        // vector table; it loads the active hardware frame and branches to
        // fault_handler_c, which has the matching C ABI.
        #[unsafe(naked)]
        #[unsafe(export_name = $symbol)]
        unsafe extern "C" fn $name() -> ! {
            naked_asm!(
                "tst lr, #4",
                "ite eq",
                "mrseq r0, msp",
                "mrsne r0, psp",
                "movs r1, {kind}",
                "b {handler}",
                kind = const $kind,
                handler = sym fault_handler_c,
            );
        }
    };
}

fault_trampoline!(memory_management, "MemoryManagement", MEM_MANAGE);
fault_trampoline!(bus_fault, "BusFault", BUS_FAULT);
fault_trampoline!(usage_fault, "UsageFault", USAGE_FAULT);
