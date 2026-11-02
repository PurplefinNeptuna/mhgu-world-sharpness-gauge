# Development provenance

Project-owned code in this folder belongs to PurplefinNeptuna (2026), all rights reserved;
see `LICENSE`.

## Vendored and linked components

- `vendor/forge-rs` — forge-rs 1.5.0, commit
  `d1865b68e9b194eb3eacc8bd71bbf601a5fac63e`, by Fexty, MIT
  (`vendor/forge-rs/LICENSE`, `licenses/FORGE-MIT.txt`). Built from source; no
  upstream binaries. It supplies the plugin lifecycle, hooks, patches, logging
  and the ImGui context used for the empty panel.
- `vendor/libnx` — libnx (Hydrosphere Developers, MIT,
  `vendor/libnx/LICENSE-MIT`). The three ARM32 SVC wrappers in `src/svc.S`
  (`svcQueryMemory`, `svcGetInfo`, `svcFlushProcessDataCache`) are adapted from
  it and keep the upstream copyright line. Forge exposes no mapping query,
  program-id query or process cache flush, which is the documented gap beside
  their use in `src/memory.rs`.
- `vendor/crates` — the exact registry sources resolved by `Cargo.lock`
  (48 directories, 23 locked packages). Runtime-linked: `log` (MIT,
  `vendor/crates/log-0.4.34/LICENSE-MIT`). Build-time only: `mhgu-forge-macros`
  and its `proc-macro2`/`quote`/`syn`/`proc-macro-crate`/`toml_*`/`winnow`
  chain, `serde_core`/`serde_derive`, `hashbrown`/`indexmap`/`equivalent`,
  `memchr`, `unicode-ident`. Each directory keeps its own license file.
- `licenses/RUST-MIT.txt`, `licenses/UNICODE-3.0.txt` and
  `licenses/COMPILER_BUILTINS.txt` cover the `core` and `alloc` built from
  source by `build-std`, including the compiler-builtins licensing exception.

Host-only toolchain, not shipped and not linked into the payload: devkitARM
binutils (`arm-none-eabi-gcc/-ld/-nm/-objdump`), `cargo-forge`, and the pinned
nightly Rust (`rust-toolchain.toml`, `dependencies.lock.json`).

## Research inputs

Offsets, signatures and decompilations for this mod live in the coordinating
workspace under `work/world-sharpness-gauge/` (`PHASE0_RESEARCH.md`,
`ghidra-batch1/`..`ghidra-batch3/`, `inputs/main`). They come from the user's
own copy of MHGU 1.4.0, are ignored by Git, and are not part of the source
export, the build recipe, or any delivered package. `scripts/verify.py` needs
`inputs/main` at verification time and ships nothing derived from it.

## AI assistance

Code, tests, build scripts and documentation in this folder were written with AI
assistance from DeepSeek Harness (https://github.com/deepseek-ai/deepseek-harness)
and Claude Code (https://claude.com/claude-code).
No offset, signature, license text or provenance claim was invented: every
address carries its proof in `DEVELOPMENT.md`, every byte window is compared
against the supplied NSO by `scripts/verify.py`, and each license file is copied
from the vendored upstream it covers. Runtime results are in `EVIDENCE.json`.
