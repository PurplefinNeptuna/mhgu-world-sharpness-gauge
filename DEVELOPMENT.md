# Developing modern-sharpness-gauge

r1 is the Phase 1 diagnostic candidate of `work/modern-sharpness-gauge/PLAN.md`.
It observes the native sharpness gauge and the local player's sharpness fields
through one hook, and writes at most 60 ERROR-level sample lines per process.
It creates no GUI objects, writes no game memory and changes nothing on screen.
Plan Phase 2 (drawing the gauge) starts only after a user run of this exact
binary has been read against these hashes.

## Commands

`./build.sh doctor|build|test|verify|package|export --output DEST`. There is no
`prepare`: this mod ships no resources, assets or INI, and has no settings
panel (`#[forge::imgui_render] fn panel() {}` stays empty so the export set
matches the proven loader shape). Inputs come from `build.local.json`
(`main`, `sdk`, `forge` under `inputs`, copied from `work/shared/game-exefs/`);
`build.local.example.json` documents the keys. Outputs go to
`build/modern-sharpness-gauge/candidates/r1/`, scratch to
`work/modern-sharpness-gauge/candidates/r1/`. `package` refuses to run for a
`READY_TO_DISTRIBUTE` spec: no distribution approval exists.

## Files

- `core/src/lib.rs` — `#![no_std]` decisions only: local-player chain, gauge
  read, fill math, gauge-HUD identity, bounded tree walk, the copied `Sample`
  and its 24 field names. No Forge, no FFI, no `unsafe`.
- `core/tests/behavior.rs` — host fixtures over a fake memory map.
- `src/hooks.rs` — the single Forge hook, signature gate, rollback.
- `src/sampling.rs` — the hot-path copy-out ring; budget, cadence, ordering.
- `src/memory.rs`, `src/svc.S` — mapping-checked reads and the three SVC
  wrappers Forge does not expose (query memory, GetInfo program id, process
  cache flush). Gap documented beside each use.
- `src/signatures.rs` — 15 32-byte windows taken from the original main NSO.
- `src/lib.rs` — metadata record, install, drain-and-format, empty panel.

## Offsets, units and proof

All values are main-module relative. Ghidra's base is `0x009c9000`; subtract it
for these. Disassembly prints words byte-swapped: the word `e92d4830` is the
memory bytes `30 48 2d e9`, so a 12-word window is written in memory order as
`30482de9 0140a0e1 ...`. `scripts/verify.py` compares memory-order bytes
against the supplied NSO; nothing here is a Ghidra-imported byte.

| Address | Meaning | Proof |
| --- | --- | --- |
| `0xa494a0` | draw walker entry; the hook site | 12-word window, unique in main (verified count 1); GE/NZCVQ audit in Phase 0 |
| `0xa49b28` | animation controller setup; sizes the draw cache from row counts | decompiled `function_00a494a0.c`/callers in `work/modern-sharpness-gauge/ghidra-batch1/` |
| `0xa39300` | instance draw; hands the controller to the walker | same batch |
| `0xa37c1c` | scissor routine writing the effective clip | batch 1 |
| `0xa4e628` | sprite prepaint; intersects live scissor with sprite rect | batch 1 |
| `0x61912c` | HUD widget driver, gauge case 8 reads the tier | batch 2 |
| `0x26893c` | `uPlayerBase::getSharpnessType`, returns `mSharpnessType` | batch 2 |
| `0x29cdfc` | `uPlayerBase::calculateSharpnessType` | batch 2 |
| `0x2ad288` | `uPlayerQuest::adjustSharpness`, the writer of `mSharpness` | batch 2 |
| `0x3f8398` | local hunter index reader over the session singleton | batch 3 |
| `0x276278` | `sPlayer::getMainPlayer` | batch 3 |
| `0xa4faf0`, `0xa4fb58` | `cGUIObjTexture` setTexture / setTextureRect | batch 3 |
| `0xa4ba20`, `0xa4ba40` | `cGUIObjScissorMask` Enable / Size setters | batch 3 |

Runtime state read by the hook:

| Location | Meaning |
| --- | --- |
| `main+0x188ca48` | session singleton; `+0x30` is the local hunter index, valid 0..3 |
| `main+0x1885b58` | `sPlayer` singleton; `+0x1c + 4*index` is the local player pointer |
| player `+10100` | `uPlayerQuest::mSharpness`, signed points |
| player `+10104` | `uPlayerQuest::mMaxSharpness`, signed points |
| player `+5104` | `uPlayerBase::mSharpnessType`, unsigned tier index (0 red .. 4) |
| controller `+0x68/+0x80/+0x8c/+0x9c/+0xa0` | tree root, draw cache, rebuild flag, animation row, owner |
| row `+0/+4/+6/+0xc/+0x14` | animation id, object count, sequence count, first object row, first sequence row |
| node `+0x38/+0x54/+0x68/+0x6c/+0x70` | rect (4 s32), flags, first child, next sibling, parent |

Node flag bits: `1` visible, `0x20000000` drawable. The gauge identity tuple,
identical in every language and in HD: animation id 4, 4 objects, first object
row 29, 15 sequences. Stamina and `pl_gauge0` rows differ, so the identity test
is the cheap filter in front of every read. The singletons live in `.bss`, past
the NSO file image, which is why `verify.py` checks the `.data/.bss` split
instead of their bytes; the chain itself is proven from the decompilations
above and is read through the mapping query at runtime.

## Hot path rules

- One hook, `e51ff004` + target, Forge relocates the first 8 bytes; install
  validates the 16-byte continuation (original bytes, branch opcode,
  `target+8`) and rolls back on any mismatch. Hook struct size 24,
  `context` at 12, `ctx_cell` at 16 (compile-time asserts).
- No `MSR` anywhere in the plugin (`verify.py` disassembly gate). The relocated
  window keeps `cmp r1,#0` with the flag-consuming `popeq`, and holds no
  SEL/USAD8/MRS/MSR, so no `APSR_g`/`APSR_nzcvqg` producer is separated from
  its consumer.
- Inside the hook: two atomic loads after the budget is spent; no allocation,
  formatting, file access, float, or logging. Reads go through
  `memory::word`, which queries the mapping per access.
- Budget: 60 samples, one every 60 walker calls, one ring slot per sample, no
  overwrite. `forge_onUpdate` drains two per frame and formats with `FIELDS`,
  hex for pointers and counts, decimal for the gauge.
- Fill math, in `core`: require a live player and a maximum `> 0`, compute in
  `i64` basis points, clamp to 0..=10000. A zero, negative, missing or
  above-maximum state yields no ratio; above maximum draws full, depletion
  floors at empty. Nothing is ever divided by an unverified maximum.

## Static evidence for r1

`./build.sh test` runs 8 host fixtures (`core/tests/behavior.rs`) then
`verify.py`. Fixtures: partial fill, empty, full, above maximum, depleted,
zero and negative maximum, extreme negative, the identity tuple against
`pl_gauge0` and stamina rows, the local-player chain (index 2, index 7, null
session, empty slot), signed gauge decode, a full controller sample carrying
150/160 at tier 2 with three drawables, a non-gauge controller costing no
sample, a gauge with no player, and a tree past the visit limit.
`verify.py` reports `PASSED`: 15 windows match the supplied NSO, the hook
window is unique (1 match), exports are exactly
`forge_onLoad/forge_onInit/forge_onUpdate/forge_onImGuiInit/forge_onImGuiRender/init/fini`,
every strong import resolves in main/sdk/forge, no settings or config import
exists, no `msr` in the disassembly, no `DT_TEXTREL`, and the payload is the
one NRO. `EVIDENCE.json` records the hashes.

## Not proven by r1

Pixel behaviour of any kind: no runtime run of this binary exists. Open items
the user run is meant to answer, in priority order:

1. Which sample lines appear at all, and how many. If `no sharpness gauge
   controller seen` appears, the walker site does not run in that state and the
   hook site is wrong.
2. `current`/`maximum` sign and unit across depletion, whetstones, meals and
   weapon changes, and whether every recovery path touches `+10100`/`+10104`
   (`adjustSharpness` is the only writer found, gates at `0x27bcd0`, `0x27c7ec`,
   `0x27b918` remain to be exercised).
3. `tier` agreement with `mSharpnessType` at `+5104` in buffed and red states.
4. `rect_*` of the first drawable: the screen direction of tip and hilt for
   `Txt_sharp` (88x24 glyph, atlas rect 0,280,88,304).
5. `visited/visible/drawable` counts, to confirm 4 objects and 3 drawables, and
   `cache_flags` behaviour on HUD recreation.
6. The `cGUI+0x2c` object map capacity and insert point, plus the 16-byte cache
   growth per inserted object (needs a Phase 2 write test, not this run).
7. Where the riding flag lives: `mInputFlags` at `+5156` and the `uPlayerQuest`
   state words are the candidates; r1 does not read them.

## Phase 2 needs

The user's log from this exact archive, matched against the NRO SHA-256 in
`DISTRIBUTION.json` and `build.json`, recorded in `EVIDENCE.json` as runtime
evidence for r1. Then the draw design can be built on measured numbers.

## Rules to keep

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/modern-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.
