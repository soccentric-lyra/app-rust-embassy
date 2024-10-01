//! MCU and board layer: register addresses and the bootloader contract.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RBT-001..006, RFN-001..005, RSAFE-001..004
#![allow(dead_code)]

pub mod boot;
pub mod clock;
pub mod fault;
pub mod identity;

/// Application link address (vector table) in the active bank (RBT-001).
pub const SLOT_BASE: u32 = 0x0800_C000;

// A-9: reserved no-init window at the top of DTCM, shared with BOOT
// (boot_layout.h BOOT_NOINIT_*).  linker/memory.x and linker/noinit.x place
// it here; the main stack starts below it.

/// No-init window start.
pub const NOINIT_ADDR: u32 = 0x2001_FC00;
/// No-init window size in bytes (1 KB).
pub const NOINIT_SIZE: u32 = 0x0000_0400;
/// No-init window end (exclusive).
pub const NOINIT_END: u32 = 0x2002_0000;
/// Boot metadata sector as mapped by BOOT.
pub const META_BASE: u32 = 0x0800_8000;
/// Address of the 32-byte CONFIRMED record.
pub const CONFIRMED_ADDR: u32 = META_BASE + 0x140;

/// Start of the flash address map.
pub const FLASH_BASE: u32 = 0x0800_0000;
/// CONFIRMED record offset for the embassy flash driver.
pub const CONFIRMED_OFFSET: u32 = CONFIRMED_ADDR - FLASH_BASE;
/// CONFIRMED record magic, "CONF".
pub const CONFIRMED_MAGIC: u32 = 0x434F_4E46;

/// RTC_BKP1R value that asks BOOT to stay in the bootloader, "STAY".
pub const STAY_IN_BOOT_MAGIC: u32 = 0x5941_5453;

// Hand-over record in RTC_BKP2R, shared with the C demo's boot_contract.h:
// bits 0..23 = magic "HND", bits 24..25 = slot, bit 26 = pending,
// bits 27..28 = trial count.

/// Hand-over magic, bytes "HND" then 0.
pub const HANDOVER_MAGIC: u32 = 0x0044_4E48;
/// Mask of the magic field.
pub const HANDOVER_MAGIC_MASK: u32 = 0x00FF_FFFF;
/// Bit position of the slot field.
pub const HANDOVER_SLOT_SHIFT: u32 = 24;
/// Slot field mask after shifting.
pub const HANDOVER_SLOT_MASK: u32 = 0x3;
/// Bit number of the trial-pending flag.
pub const HANDOVER_PENDING_BIT: u32 = 26;
/// Bit position of the trial count field.
pub const HANDOVER_TRIAL_SHIFT: u32 = 27;
/// Trial count field mask after shifting.
pub const HANDOVER_TRIAL_MASK: u32 = 0x3;

/// Slot not known (no valid hand-over).
pub const SLOT_UNKNOWN: u8 = 0;
/// Slot A.
pub const SLOT_A: u8 = 1;
/// Slot B.
pub const SLOT_B: u8 = 2;

/// Trial state not known.
pub const TRIAL_UNKNOWN: u8 = 0;
/// Slot is confirmed.
pub const TRIAL_CONFIRMED: u8 = 1;
/// Slot is on trial and must pass the health check.
pub const TRIAL_PENDING: u8 = 2;

/// No health check needed (slot not on trial).
pub const HEALTH_NOT_REQUIRED: u8 = 0;
/// Health check in progress.
pub const HEALTH_RUNNING: u8 = 1;
/// Health check passed; CONFIRMED written.
pub const HEALTH_PASSED: u8 = 2;
/// Health check failed.
pub const HEALTH_FAILED: u8 = 3;

/// RCC clock control and status register.
pub const RCC_CSR: *mut u32 = 0x4002_3874 as *mut u32;
/// RCC_CSR: remove reset flags.
pub const RCC_CSR_RMVF: u32 = 1 << 24;
/// RCC_CSR: brown-out reset.
pub const RCC_CSR_BORRSTF: u32 = 1 << 25;
/// RCC_CSR: NRST pin reset.
pub const RCC_CSR_PINRSTF: u32 = 1 << 26;
/// RCC_CSR: power-on reset.
pub const RCC_CSR_PORRSTF: u32 = 1 << 27;
/// RCC_CSR: software reset.
pub const RCC_CSR_SFTRSTF: u32 = 1 << 28;
/// RCC_CSR: independent watchdog reset.
pub const RCC_CSR_WDGRSTF: u32 = 1 << 29;
/// RCC_CSR: window watchdog reset.
pub const RCC_CSR_WWDGRSTF: u32 = 1 << 30;
/// RCC_CSR: low-power reset.
pub const RCC_CSR_LPWRRSTF: u32 = 1 << 31;
/// All RCC_CSR reset-cause flags.
pub const RCC_CSR_RESET_MASK: u32 = RCC_CSR_BORRSTF
    | RCC_CSR_PINRSTF
    | RCC_CSR_PORRSTF
    | RCC_CSR_SFTRSTF
    | RCC_CSR_WDGRSTF
    | RCC_CSR_WWDGRSTF
    | RCC_CSR_LPWRRSTF;

/// IWDG key register.
pub const IWDG_KR: *mut u32 = 0x4000_3000 as *mut u32;
/// IWDG_KR value that reloads the counter.
pub const IWDG_KEY_RELOAD: u32 = 0xAAAA;

/// RCC APB1 peripheral clock enable register.
pub const RCC_APB1ENR: *mut u32 = 0x4002_3840 as *mut u32;
/// RCC_APB1ENR: PWR clock enable.
pub const RCC_APB1ENR_PWREN: u32 = 1 << 28;
/// RCC_APB1ENR: RTC APB clock enable.
pub const RCC_APB1ENR_RTCEN: u32 = 1 << 10;
/// PWR control register 1.
pub const PWR_CR: *mut u32 = 0x4000_7000 as *mut u32;
/// PWR_CR: backup domain write enable.
pub const PWR_CR_DBP: u32 = 1 << 8;
/// RTC backup register 0.
pub const RTC_BKP0R: *mut u32 = 0x4000_2850 as *mut u32;
/// RTC backup register 1 (stay-in-BOOT request).
pub const RTC_BKP1R: *mut u32 = 0x4000_2854 as *mut u32;
/// RTC backup register 2 (hand-over record).
pub const RTC_BKP2R: *mut u32 = 0x4000_2858 as *mut u32;
/// RTC write protection register.
pub const RTC_WPR: *mut u32 = 0x4000_2824 as *mut u32;
/// First RTC_WPR unlock key.
pub const RTC_WPR_UNLOCK1: u32 = 0x00CA;
/// Second RTC_WPR unlock key.
pub const RTC_WPR_UNLOCK2: u32 = 0x0053;
/// RTC_WPR value that re-enables write protection.
pub const RTC_WPR_LOCK: u32 = 0xFF;

/// DBGMCU device ID code register.
pub const DBGMCU_IDCODE: *const u32 = 0xE004_2000 as *const u32;
/// STM32F76x/77x device ID, reported when DBGMCU_IDCODE reads 0.
pub const DBGMCU_DEV_ID: u32 = 0x451;
/// 96-bit unique device ID (three words).
pub const UID_BASE: *const u32 = 0x1FF0_F420 as *const u32;

/// RCC AHB1 peripheral clock enable register.
pub const RCC_AHB1ENR: *mut u32 = 0x4002_3830 as *mut u32;
/// RCC_AHB1ENR: GPIOB clock enable.
pub const RCC_AHB1ENR_GPIOBEN: u32 = 1 << 1;
/// GPIOB mode register.
pub const GPIOB_MODER: *mut u32 = 0x4002_0400 as *mut u32;
/// GPIOB bit set/reset register.
pub const GPIOB_BSRR: *mut u32 = 0x4002_0418 as *mut u32;
/// LD3 (red) pin number on GPIOB.
pub const LD3_PIN: u32 = 14;
