# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Footprint gate (128 KB flash, 64 KB RAM) and a boot-contract check, matching
  the other four repositories
- Host test crate for `src/protocol/frame.rs`: COBS examples, block
  boundaries, CRC corruption, reserved flags and size limits
- `cargo audit` and `cargo deny` jobs plus a `deny.toml` policy
- `docs/architecture.md`: executor/task model, IPC channels, boot hand-over,
  memory map and the USART6 debug flow
- `make docs` builds rustdoc with private items; GitHub Actions publishes it
- SAFETY comments on the remaining unsafe attributes and exception handlers;
  `clippy::undocumented_unsafe_blocks` is denied

### Changed

- `scripts/build.sh` now enforces the footprint budget after linking
- The Jenkins image stage reuses `scripts/check_image.py` instead of an inline
  copy of the header checks

## [1.0.0] - 2026-11-19

### Added

- Demo application in Rust on the Embassy async framework: LED patterns,
  the user button, HWLINK over UART, watchdog supervision and self-confirmation
- Safe, allocation-free (`no_std`) application on top of the existing C
  bootloader and toolchain
- Behaviour matches the C FreeRTOS demo for a like-for-like comparison
- Runs from either A/B slot under the bootloader

[Unreleased]: https://github.com/soccentric-stm32/app-rust-embassy/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/soccentric-stm32/app-rust-embassy/releases/tag/v1.0.0
