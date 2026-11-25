//! Host harness for the firmware's HWLINK framing module.
//!
//! `frame.rs` has no architecture dependencies: it is plain COBS and CRC-32
//! over byte slices.  Including it here compiles the same source the firmware
//! uses and lets `cargo test` run it on the host.

#[path = "../../../src/protocol/frame.rs"]
pub mod frame;
