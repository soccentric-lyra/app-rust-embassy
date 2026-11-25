# Planned improvements: app-rust-embassy

Gaps found in review and the enhancements planned to close them, highest priority first. Priority 1 is next, 3 is later.

## Gaps

| Area | Gap |
|---|---|
| CI parity | No footprint gate and no boot-contract stage. The other four have both. This is the weakest CI of the set. |
| Tests | No `#[test]` anywhere, although `frame.rs` (COBS, CRC) is pure logic. |
| Docs | The README only. There is no `cargo doc`, and no explanation of the Embassy task and executor model. |
| `unsafe` | Five `unsafe` blocks have no SAFETY comment. |
| Duplication | `frame.rs` (231 lines) differs from the copies in the other two Rust repos, so the protocol code has already diverged. |
| Dependencies | No `cargo audit` or `cargo deny` check. |
| Validation | No emulated run and no `defmt` or RTT tracing documented. |

## Enhancements

| Priority | Enhancement | Result |
|---|---|---|
| 1 | Add `check_footprint.py` (copy from the other repos) and the boot-contract CI stage | CI parity with the set |
| 1 | Split the protocol into a `no_std` crate with `#[test]` tests that run on the host, and share it with the two other Rust repos | Tested, single source of truth |
| 1 | GitHub Actions workflow: build, clippy, fmt, footprint, `cargo test`, badge | Visible green status |
| 2 | Add SAFETY comments to the remaining five `unsafe` blocks and enforce it with `clippy::undocumented_unsafe_blocks` | Rust discipline is checked, not claimed |
| 2 | `cargo audit` and `cargo deny` in CI | Supply-chain evidence |
| 2 | `docs/architecture.md`: executor and task model, channels in `system/ipc.rs`, how it hands over to and from the bootloader. Publish `cargo doc`. | Shows the async design |
| 3 | Add `defmt` + RTT as an option and document the debug flow | Modern Rust debugging |
| 3 | Add a memory-safety angle: `cargo geiger` count compared with the C versions | A concrete Rust vs C point |

