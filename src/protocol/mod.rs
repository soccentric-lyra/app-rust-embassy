//! HWLINK demo message set (schema: `protocol/hwlink.fbs`).
//!
//! See [`fb`] for why the FlatBuffers code is local (OPEN-R01).  The
//! builders return `Option` but panic, through [`fb::Builder`], when `buf`
//! is too small.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RFN-003, RFN-005, RLNK-001..004; LINK A.3
#![allow(dead_code)]

pub mod fb;
pub mod frame;

use fb::{Builder, Table, Value};

/// Protocol minor version reported in `HelloReply` and `Status`.
pub const PROTOCOL_MINOR: u16 = 1;
/// Largest raw frame in bytes, reported in `HelloReply`.
pub const MAX_FRAME: usize = frame::MAX_DECODED;

/// Union tags of `Message.body`; the order is the wire format (LINK-013).
pub mod body {
    /// No body.
    pub const NONE: u8 = 0;
    /// `Hello` request.
    pub const HELLO: u8 = 1;
    /// `HelloReply` response.
    pub const HELLO_REPLY: u8 = 2;
    /// `Ping` request.
    pub const PING: u8 = 3;
    /// `Pong` response.
    pub const PONG: u8 = 4;
    /// `SetBaud` request.
    pub const SET_BAUD: u8 = 5;
    /// `Ack` response.
    pub const ACK: u8 = 6;
    /// `Nak` response.
    pub const NAK: u8 = 7;
    /// `Log` event.
    pub const LOG: u8 = 8;
    /// `Progress` event.
    pub const PROGRESS: u8 = 9;
    /// `Reset` request.
    pub const RESET: u8 = 10;
    /// `GetBootInfo` request (BOOT only).
    pub const GET_BOOT_INFO: u8 = 11;
    /// `BootInfo` response (BOOT only).
    pub const BOOT_INFO: u8 = 12;
    /// `UpdateBegin` request (BOOT only).
    pub const UPDATE_BEGIN: u8 = 13;
    /// `UpdateData` request (BOOT only).
    pub const UPDATE_DATA: u8 = 14;
    /// `UpdateEnd` request (BOOT only).
    pub const UPDATE_END: u8 = 15;
    /// `UpdateResult` response (BOOT only).
    pub const UPDATE_RESULT: u8 = 16;
    /// `Activate` request (BOOT only).
    pub const ACTIVATE: u8 = 17;
    /// `UpdateAbort` request (BOOT only).
    pub const UPDATE_ABORT: u8 = 18;
    /// `SetLed` request.
    pub const SET_LED: u8 = 19;
    /// `SetLedAll` request.
    pub const SET_LED_ALL: u8 = 20;
    /// `ClearFault` request.
    pub const CLEAR_FAULT: u8 = 21;
    /// `EnterBootloader` request.
    pub const ENTER_BOOTLOADER: u8 = 22;
    /// `GetStatus` request.
    pub const GET_STATUS: u8 = 23;
    /// `Status` response.
    pub const STATUS: u8 = 24;
    /// `ButtonEvent` event.
    pub const BUTTON_EVENT: u8 = 25;
}

/// `Role` enum: firmware role in `HelloReply`.
pub mod role {
    /// Hardware validation firmware.
    pub const HWVAL: u8 = 1;
    /// Bootloader.
    pub const BOOTLOADER: u8 = 2;
    /// Demo application (this firmware).
    pub const DEMO: u8 = 3;
}

/// `FlashMode` enum.
pub mod flash_mode {
    /// Not known.
    pub const UNKNOWN: u8 = 0;
    /// Single-bank flash.
    pub const SINGLE_BANK: u8 = 1;
    /// Dual-bank flash.
    pub const DUAL_BANK: u8 = 2;
}

/// `Slot` enum.
pub mod slot {
    /// Not known.
    pub const UNKNOWN: u8 = 0;
    /// Slot A.
    pub const A: u8 = 1;
    /// Slot B.
    pub const B: u8 = 2;
}

/// `TrialState` enum.
pub mod trial {
    /// Not known.
    pub const UNKNOWN: u8 = 0;
    /// Slot confirmed.
    pub const CONFIRMED: u8 = 1;
    /// Slot on trial.
    pub const PENDING: u8 = 2;
}

/// `HealthState` enum.
pub mod health {
    /// No health check needed.
    pub const NOT_REQUIRED: u8 = 0;
    /// Health check in progress.
    pub const RUNNING: u8 = 1;
    /// Health check passed.
    pub const PASSED: u8 = 2;
    /// Health check failed.
    pub const FAILED: u8 = 3;
}

/// `ButtonPress` enum.
pub mod button {
    /// Short press.
    pub const SHORT: u8 = 1;
    /// Long press.
    pub const LONG: u8 = 2;
}

/// `LedPattern` enum (Appendix A).
pub mod pattern {
    /// Off.
    pub const OFF: u8 = 0;
    /// On.
    pub const ON: u8 = 1;
    /// 50 % blink at the period.
    pub const BLINK: u8 = 2;
    /// Double flash per period.
    pub const HEARTBEAT: u8 = 3;
    /// Blink at a fifth of the period.
    pub const FAST: u8 = 4;
    /// LD1 and LD2 alternate.
    pub const CHASE: u8 = 5;
    /// SOS in Morse code.
    pub const MORSE_SOS: u8 = 6;

    /// Number of patterns; valid values are below it.
    pub const COUNT: u8 = 7;

    /// Return whether `v` is a known pattern.
    pub fn valid(v: u8) -> bool {
        v < COUNT
    }
}

/// `LedId` enum.
pub mod led {
    /// LD1 (green).
    pub const LD1: u8 = 1;
    /// LD2 (blue).
    pub const LD2: u8 = 2;
    /// LD3 (red), reserved for faults unless forced.
    pub const LD3: u8 = 3;
}

/// `Error` enum: `Nak` error codes.
pub mod error {
    /// No error.
    pub const NONE: u16 = 0;
    /// Message failed to decode or encode.
    pub const MALFORMED: u16 = 1;
    /// Message type not handled.
    pub const UNSUPPORTED: u16 = 2;
    /// Argument out of range.
    pub const BAD_ARGUMENT: u16 = 3;
    /// Not allowed in the current state.
    pub const BAD_STATE: u16 = 4;
    /// Resource busy (for example, LED queue full).
    pub const BUSY: u16 = 5;
    /// Item not found.
    pub const NOT_FOUND: u16 = 6;
}

/// Request decode failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Buffer or table structure is invalid.
    Malformed,
    /// Unsupported message; `parse_request` does not return it.
    Unsupported,
}

/// Decoded host request.
pub enum Request<'a> {
    /// Open a session.
    Hello {
        /// Host program name.
        host_name: Option<&'a str>,
        /// Host program version.
        host_version: Option<&'a str>,
        /// Host protocol minor version; 0 when absent.
        protocol_minor: u16,
    },
    /// Liveness check.
    Ping,
    /// Ask for `Status`.
    GetStatus,
    /// Set one LED; absent fields read as LD1, OFF, 0, false.
    SetLed {
        /// `led::*` target.
        led: u8,
        /// `pattern::*` value.
        pattern: u8,
        /// Period in ms.
        period_ms: u16,
        /// Allow LD3.
        force: bool,
    },
    /// Set LD1 and LD2.
    SetLedAll {
        /// `pattern::*` value.
        pattern: u8,
        /// Period in ms.
        period_ms: u16,
    },
    /// Clear the fault record and LD3 fault indication.
    ClearFault,
    /// Reset into BOOT.
    EnterBootloader,
    /// Reset the MCU.
    Reset,
    /// Change the UART baud rate.
    SetBaud {
        /// Baud rate in bit/s; 0 when absent.
        rate: u32,
    },
    /// Any other body type.
    Unknown,
}

/// Read a required `ubyte` field.
fn required_u8(t: Table<'_>, id: u8) -> Result<u8, ParseError> {
    t.u8(id).ok_or(ParseError::Malformed)
}

/// Decode a `Message` payload into a [`Request`].
///
/// # Errors
///
/// [`ParseError::Malformed`] for a bad buffer, a missing body type, or a
/// missing body where one is required.
pub fn parse_request(payload: &[u8]) -> Result<Request<'_>, ParseError> {
    let msg = Table::root(payload, fb::FILE_ID).ok_or(ParseError::Malformed)?;
    let body_type = required_u8(msg, 0)?;
    let body = if body_type == body::NONE {
        None
    } else {
        Some(msg.table(1).ok_or(ParseError::Malformed)?)
    };

    Ok(match body_type {
        body::HELLO => {
            let t = body.ok_or(ParseError::Malformed)?;
            Request::Hello {
                host_name: t.str(0),
                host_version: t.str(1),
                protocol_minor: t.u16(2).unwrap_or(0),
            }
        }
        body::PING => Request::Ping,
        body::GET_STATUS => Request::GetStatus,
        body::SET_LED => {
            let t = body.ok_or(ParseError::Malformed)?;
            // FlatBuffers omits scalar fields equal to their default.  The
            // generated C reader returns LED1/OFF/0 in that case, so do the
            // same and let the argument checks answer BAD_ARGUMENT.
            Request::SetLed {
                led: t.u8(0).unwrap_or(led::LD1),
                pattern: t.u8(1).unwrap_or(pattern::OFF),
                period_ms: t.u16(2).unwrap_or(0),
                force: t.u8(3).unwrap_or(0) != 0,
            }
        }
        body::SET_LED_ALL => {
            let t = body.ok_or(ParseError::Malformed)?;
            Request::SetLedAll {
                pattern: t.u8(0).unwrap_or(pattern::OFF),
                period_ms: t.u16(1).unwrap_or(0),
            }
        }
        body::CLEAR_FAULT => Request::ClearFault,
        body::ENTER_BOOTLOADER => Request::EnterBootloader,
        body::RESET => Request::Reset,
        body::SET_BAUD => Request::SetBaud {
            rate: body.ok_or(ParseError::Malformed)?.u32(0).unwrap_or(0),
        },
        _ => Request::Unknown,
    })
}

/// Fields of a `HelloReply`; role and protocol versions are added by
/// [`hello_reply`].
pub struct HelloReply<'a> {
    /// Firmware name.
    pub firmware_name: &'a str,
    /// Version text.
    pub version: &'a str,
    /// Git commit.
    pub commit: &'a str,
    /// Build time (UTC).
    pub build_time: &'a str,
    /// 64-bit device UID.
    pub uid: u64,
    /// DBGMCU device ID.
    pub dev_id: u32,
    /// DBGMCU revision ID.
    pub rev_id: u32,
    /// `flash_mode::*` value.
    pub flash_mode: u8,
    /// Free-form detail (factory identity).
    pub detail: &'a str,
}

/// Encode a `HelloReply` message into `buf`.
pub fn hello_reply<'a>(buf: &'a mut [u8], info: &HelloReply<'_>) -> Option<&'a [u8]> {
    let mut b = Builder::new(buf);
    let name = b.string(info.firmware_name);
    let version = b.string(info.version);
    let commit = b.string(info.commit);
    let build_time = b.string(info.build_time);
    let detail = b.string(info.detail);
    let t = b.table(&[
        (0, Value::U8(role::DEMO)),
        (1, Value::Offset(name)),
        (2, Value::Offset(version)),
        (3, Value::Offset(commit)),
        (4, Value::Offset(build_time)),
        (5, Value::U64(info.uid)),
        (6, Value::U32(info.dev_id)),
        (7, Value::U32(info.rev_id)),
        (8, Value::U8(info.flash_mode)),
        (9, Value::U16(frame::PROTO_MAJOR as u16)),
        (10, Value::U16(PROTOCOL_MINOR)),
        (11, Value::U16(MAX_FRAME as u16)),
        (12, Value::Offset(detail)),
    ]);
    let m = b.message(body::HELLO_REPLY, t);
    Some(b.finish(m))
}

/// Encode a `Pong` with the uptime in ms.
pub fn pong(buf: &mut [u8], uptime_ms: u64) -> Option<&[u8]> {
    let mut b = Builder::new(buf);
    let t = b.table(&[(0, Value::U64(uptime_ms))]);
    let m = b.message(body::PONG, t);
    Some(b.finish(m))
}

/// Fields of a `FaultInfo` table.
pub struct FaultInfoData<'a> {
    /// A fault record exists.
    pub present: bool,
    /// Fault kind name.
    pub reason: &'a str,
    /// SCB CFSR.
    pub cfsr: u32,
    /// SCB HFSR.
    pub hfsr: u32,
    /// SCB MMFAR.
    pub mmfar: u32,
    /// SCB BFAR.
    pub bfar: u32,
    /// Stacked PC.
    pub pc: u32,
    /// Stacked LR.
    pub lr: u32,
    /// Task name.
    pub task: &'a str,
    /// Panic source file.
    pub file: &'a str,
    /// Panic source line.
    pub line: u32,
}

/// Fields of a `LedState` table; the LED id comes from the array index.
pub struct LedStateData {
    /// `pattern::*` value.
    pub pattern: u8,
    /// Period in ms.
    pub period_ms: u16,
}

/// Fields of a `TaskStats` table.
pub struct TaskStatsData<'a> {
    /// Task name.
    pub name: &'a str,
    /// Priority as reported to the host.
    pub priority: u8,
    /// Task state code.
    pub state: u8,
    /// Stack high-water mark in bytes.
    pub stack_high_water: u16,
}

/// Fields of a `Status`; protocol versions are added by [`status`].
pub struct Status<'a> {
    /// Firmware name.
    pub firmware_name: &'a str,
    /// Version text.
    pub version: &'a str,
    /// Git commit.
    pub commit: &'a str,
    /// Build time (UTC).
    pub build_time: &'a str,
    /// 64-bit device UID.
    pub uid: u64,
    /// Uptime in ms.
    pub uptime_ms: u64,
    /// `slot::*` value.
    pub slot: u8,
    /// `trial::*` value.
    pub trial: u8,
    /// Trial attempts counted by BOOT.
    pub trial_count: u8,
    /// `health::*` value.
    pub health: u8,
    /// RCC_CSR reset flags.
    pub reset_cause: u32,
    /// Reset cause text.
    pub reset_cause_text: &'a str,
    /// Fault record; omitted when `None`.
    pub fault: Option<FaultInfoData<'a>>,
    /// LD1..LD3 state.
    pub leds: [LedStateData; 3],
    /// Task statistics; at most 8 are encoded.
    pub tasks: &'a [TaskStatsData<'a>],
    /// Free heap in bytes (0: no heap).
    pub heap_free: u32,
    /// Lowest free heap in bytes (0: no heap).
    pub heap_min_free: u32,
}

/// Encode a `Status` message into `buf`.
pub fn status<'a>(buf: &'a mut [u8], s: &Status<'_>) -> Option<&'a [u8]> {
    let mut b = Builder::new(buf);

    let firmware_name = b.string(s.firmware_name);
    let version = b.string(s.version);
    let commit = b.string(s.commit);
    let build_time = b.string(s.build_time);
    let reset_cause_text = b.string(s.reset_cause_text);

    let fault = s.fault.as_ref().map(|f| {
        let reason = b.string(f.reason);
        let task = b.string(f.task);
        let file = b.string(f.file);
        b.table(&[
            (0, Value::U8(f.present as u8)),
            (1, Value::Offset(reason)),
            (2, Value::U32(f.cfsr)),
            (3, Value::U32(f.hfsr)),
            (4, Value::U32(f.mmfar)),
            (5, Value::U32(f.bfar)),
            (6, Value::U32(f.pc)),
            (7, Value::U32(f.lr)),
            (8, Value::Offset(task)),
            (9, Value::Offset(file)),
            (10, Value::U32(f.line)),
        ])
    });

    let mut led_refs = [0usize; 3];
    for (i, led) in s.leds.iter().enumerate() {
        led_refs[i] = b.table(&[
            (0, Value::U8(led::LD1 + i as u8)),
            (1, Value::U8(led.pattern)),
            (2, Value::U16(led.period_ms)),
        ]);
    }
    let leds = b.offset_vector(&led_refs);

    let mut task_refs = [0usize; 8];
    for (i, task) in s.tasks.iter().enumerate().take(task_refs.len()) {
        let name = b.string(task.name);
        task_refs[i] = b.table(&[
            (0, Value::Offset(name)),
            (1, Value::U8(task.priority)),
            (2, Value::U8(task.state)),
            (3, Value::U16(task.stack_high_water)),
        ]);
    }
    let tasks = b.offset_vector(&task_refs[..s.tasks.len().min(task_refs.len())]);

    let mut fields = [(0u8, Value::U8(0)); 19];
    let mut n = 0usize;
    fields[n] = (0, Value::Offset(firmware_name));
    n += 1;
    fields[n] = (1, Value::Offset(version));
    n += 1;
    fields[n] = (2, Value::Offset(commit));
    n += 1;
    fields[n] = (3, Value::Offset(build_time));
    n += 1;
    fields[n] = (4, Value::U64(s.uid));
    n += 1;
    fields[n] = (5, Value::U64(s.uptime_ms));
    n += 1;
    fields[n] = (6, Value::U8(s.slot));
    n += 1;
    fields[n] = (7, Value::U8(s.trial));
    n += 1;
    fields[n] = (8, Value::U8(s.trial_count));
    n += 1;
    fields[n] = (9, Value::U8(s.health));
    n += 1;
    fields[n] = (10, Value::U32(s.reset_cause));
    n += 1;
    fields[n] = (11, Value::Offset(reset_cause_text));
    n += 1;
    if let Some(fault) = fault {
        fields[n] = (12, Value::Offset(fault));
        n += 1;
    }
    fields[n] = (13, Value::Offset(leds));
    n += 1;
    fields[n] = (14, Value::Offset(tasks));
    n += 1;
    fields[n] = (15, Value::U32(s.heap_free));
    n += 1;
    fields[n] = (16, Value::U32(s.heap_min_free));
    n += 1;
    fields[n] = (17, Value::U16(frame::PROTO_MAJOR as u16));
    n += 1;
    fields[n] = (18, Value::U16(PROTOCOL_MINOR));
    n += 1;

    let t = b.table(&fields[..n]);
    let m = b.message(body::STATUS, t);
    Some(b.finish(m))
}

/// Encode an `Ack` for `seq`.
pub fn ack(buf: &mut [u8], seq: u16) -> Option<&[u8]> {
    let mut b = Builder::new(buf);
    let t = b.table(&[(0, Value::U16(seq)), (1, Value::U32(0))]);
    let m = b.message(body::ACK, t);
    Some(b.finish(m))
}

/// Encode a `Nak` for `seq` with error `code` and `detail`.
pub fn nak<'a>(buf: &'a mut [u8], seq: u16, code: u16, detail: &str) -> Option<&'a [u8]> {
    let mut b = Builder::new(buf);
    let detail = b.string(detail);
    let t = b.table(&[
        (0, Value::U16(seq)),
        (1, Value::U16(code)),
        (2, Value::Offset(detail)),
        (3, Value::U32(0)),
    ]);
    let m = b.message(body::NAK, t);
    Some(b.finish(m))
}

/// Encode a `ButtonEvent`.
pub fn button_event(buf: &mut [u8], press: u8, pattern: u8, period_ms: u16) -> Option<&[u8]> {
    let mut b = Builder::new(buf);
    let t = b.table(&[
        (0, Value::U8(press)),
        (1, Value::U8(pattern)),
        (2, Value::U16(period_ms)),
    ]);
    let m = b.message(body::BUTTON_EVENT, t);
    Some(b.finish(m))
}
