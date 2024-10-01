//! Channels, status watch and check-in counters shared by the four tasks.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RRT-003, RRT-004; DRT-011

use core::sync::atomic::{AtomicU32, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::Watch;

/// Check-in index of the LED task.
pub const TASK_LED: usize = 0;
/// Check-in index of the button task.
pub const TASK_BUTTON: usize = 1;
/// Check-in index of the comms task.
pub const TASK_COMMS: usize = 2;
/// Check-in index of the health task.
pub const TASK_HEALTH: usize = 3;

/// Live state reported in `Status`, published through [`STATUS`].
#[derive(Clone, Copy)]
pub struct StatusSnapshot {
    /// LD1 pattern, `protocol::pattern::*`.
    pub led1_pattern: u8,
    /// LD1 period in ms.
    pub led1_period: u16,
    /// LD2 pattern, `protocol::pattern::*`.
    pub led2_pattern: u8,
    /// LD2 period in ms.
    pub led2_period: u16,
    /// `board::HEALTH_*` state.
    pub health_state: u8,
    /// A fault record or watchdog reset is latched (LD3 on).
    pub fault_active: bool,
    /// The health check failed.
    pub health_failed: bool,
    /// Total task check-ins counted by the health task.
    pub task_checks: u32,
    /// Main stack high-water mark in bytes.
    pub stack_high_water: u32,
    /// LD3 pattern, `protocol::pattern::*`.
    pub led3_pattern: u8,
    /// LD3 period in ms.
    pub led3_period: u16,
}

impl StatusSnapshot {
    /// Power-on defaults: LD1 heartbeat at 1000 ms, LD2/LD3 off.
    pub const fn initial() -> Self {
        Self {
            led1_pattern: crate::protocol::pattern::HEARTBEAT,
            led1_period: 1000,
            led2_pattern: crate::protocol::pattern::OFF,
            led2_period: 1000,
            health_state: crate::board::HEALTH_NOT_REQUIRED,
            fault_active: false,
            health_failed: false,
            task_checks: 0,
            stack_high_water: 0,
            led3_pattern: crate::protocol::pattern::OFF,
            led3_period: 1000,
        }
    }
}

/// Latest [`StatusSnapshot`]; read by the comms task for `GetStatus`.
pub static STATUS: Watch<CriticalSectionRawMutex, StatusSnapshot, 2> =
    Watch::new_with(StatusSnapshot::initial());

/// Command for the LED task, which owns all LED state.
#[derive(Clone, Copy)]
pub enum LedCommand {
    /// Set one LED (`SetLed`).
    Set {
        /// `protocol::led::*` target.
        led: u8,
        /// `protocol::pattern::*` value.
        pattern: u8,
        /// Period in ms.
        period_ms: u16,
        /// Allow LD3, which is otherwise reserved for faults.
        force: bool,
    },
    /// Set LD1 and LD2 (`SetLedAll`).
    SetAll {
        /// `protocol::pattern::*` value.
        pattern: u8,
        /// Period in ms.
        period_ms: u16,
    },
    /// Short press: advance to the next pattern (BTN-002).
    ButtonShort,
    /// Long press: restore the default patterns (BTN-003).
    ButtonLong,
    /// Drop the fault indication on LD3.
    ClearFault,
    /// Health check failed: fast-blink LD3 (LED-005).
    HealthFailed,
}

/// Commands to the LED task; senders use `try_send` and never block.
pub static LED_COMMANDS: Channel<CriticalSectionRawMutex, LedCommand, 8> = Channel::new();

/// [`ButtonEvent::press`] value for a short press.
pub const BUTTON_SHORT: u8 = 1;
/// [`ButtonEvent::press`] value for a long press.
pub const BUTTON_LONG: u8 = 2;

/// Button result sent to the host as a `ButtonEvent` message.
#[derive(Clone, Copy)]
pub struct ButtonEvent {
    /// [`BUTTON_SHORT`] or [`BUTTON_LONG`].
    pub press: u8,
    /// LD1/LD2 pattern after the press.
    pub pattern: u8,
    /// LD1/LD2 period in ms after the press.
    pub period_ms: u16,
}

/// Button events from the LED task to the comms task.
pub static BUTTON_EVENTS: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/// Check-in counters, one per `TASK_*` index.
///
/// Counters, not a `Signal`: a signal would collapse several check-ins
/// between two health polls into one and make the 5 s pass depend on timing.
static CHECKINS: [AtomicU32; 4] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

/// Count one check-in for `task` (a `TASK_*` index); call once per loop
/// (RRT-004).
///
/// # Panics
///
/// Panics if `task` is 4 or more.
pub fn check_in(task: usize) {
    CHECKINS[task].fetch_add(1, Ordering::Relaxed);
}

/// Return and reset the check-in count of `task` since the last call.
///
/// # Panics
///
/// Panics if `task` is 4 or more.
pub fn take_check_in(task: usize) -> u32 {
    CHECKINS[task].swap(0, Ordering::Relaxed)
}
