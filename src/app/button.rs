//! B1 (PC13) debounce and short/long press detection.
//!
//! The task sends one command per press; the LED task owns the pattern state
//! (as in the C demo), so a second press queued before the first is handled
//! never loses a step.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RFN-002; BTN-001..BTN-004

use embassy_futures::select::select;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::mode::Async;
use embassy_time::{Duration, Instant, Timer};

use crate::system::ipc::{self, LedCommand};

/// Time the level must be stable to count as a change, in ms.
const DEBOUNCE_MS: u64 = 30;
/// Longest hold that still counts as a short press, in ms.
const SHORT_MAX_MS: u64 = 1000;
/// Hold time that triggers a long press, in ms (BTN-003).
const LONG_MIN_MS: u64 = 2000;
/// Poll interval while waiting for an edge.
const POLL: Duration = Duration::from_millis(10);
/// Interval between health check-ins, in ms.
const CHECKIN_PERIOD_MS: u64 = 500;

/// Button task: debounce B1 and send `ButtonShort`/`ButtonLong` commands.
///
/// A release between 1000 and 2000 ms sends nothing.
#[embassy_executor::task]
pub async fn task(mut btn: ExtiInput<'static, Async>) {
    // B1 is high when pressed and has an external pull-down on the board.
    let mut pressed = btn.is_high();
    let mut candidate = pressed;
    let mut candidate_since = Instant::now();
    let mut press_start = Instant::now();
    let mut long_sent = false;
    let mut last_checkin = Instant::now();
    let mut report = crate::system::log::TaskReport::new(
        2,
        "Button",
        "debounces B1, short press cycles the LED pattern, long press resets it",
    );

    loop {
        report.tick();
        // EXTI wakes us on an edge; the timeout keeps the debounce loop and
        // the health check-in running while the button is held.
        let _ = select(btn.wait_for_any_edge(), Timer::after(POLL)).await;

        let now = Instant::now();
        let raw = btn.is_high();
        if raw != candidate {
            candidate = raw;
            candidate_since = now;
        } else if candidate != pressed
            && now.duration_since(candidate_since).as_millis() >= DEBOUNCE_MS
        {
            pressed = candidate;
            if pressed {
                press_start = now;
                long_sent = false;
            } else {
                let held = now.duration_since(press_start).as_millis();
                if !long_sent && held < SHORT_MAX_MS {
                    log_info!("button: short press, next LED pattern");
                    let _ = ipc::LED_COMMANDS.try_send(LedCommand::ButtonShort);
                }
            }
        }

        // BTN-003: a long press acts at 2 s, not only when the button is
        // released, exactly as the C demo's button task does.
        if pressed && !long_sent && now.duration_since(press_start).as_millis() >= LONG_MIN_MS {
            long_sent = true;
            log_info!("button: long press, default LED patterns");
            let _ = ipc::LED_COMMANDS.try_send(LedCommand::ButtonLong);
        }

        if now.duration_since(last_checkin).as_millis() >= CHECKIN_PERIOD_MS {
            ipc::check_in(ipc::TASK_BUTTON);
            last_checkin = now;
        }
    }
}
