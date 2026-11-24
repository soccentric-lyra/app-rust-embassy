# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
