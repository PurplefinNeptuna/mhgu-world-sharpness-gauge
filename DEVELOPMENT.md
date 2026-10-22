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
| row `+0/+4/+0xc/+0x14` | animation id; counts packed in one word (objects low u16, sequences high u16); first object row; the word at `+0x14` is a pointer into the sequence block, not a row index |
| node `+0x33` / `+0x42` / `+0x54` | kind word (`& 0xf`), draw-dirty byte (`& 0x30`), flag word `+0x54..+0x57` |
| node `+0x68/+0x6c/+0x70` | first child, next sibling, parent |
| node `+0xE0..+0xEC` / `+0xF0..+0xFC` | own screen rect (4 signed ints), then the rect prepaint leaves after the live-clip intersection |
| node `+0x100/+0x104/+0x10C..+0x118` | texture id, texture object, atlas rect (4 ints, identifies the glyph) |
| context `+0xD0..+0xDC` | the live clip every drawable in this walk is intersected with |

Ghidra indexes `int *` members in WORDS, so a decompiled `param_1[0x38]` sits
at byte offset `0x38 * 4 = 0xE0`, while written pointer arithmetic (`param_1 + 0x54`)
is already a byte offset; that is the rule behind the rect offsets above. The node
flag word at `+0x54` covers bytes `+0x54..+0x57`: bit 0 visible (the walker skips an
invisible subtree), `0x0200_0000` drawable, `0x0040_0000` leaf exit (byte `+0x57` bit
1), `0x0000_4000` prepaint skip (byte `+0x56` bit 6). The r1 walk reported
`drawable=0` because r1 tested `0x2000_0000`; r2 tests the value the walker itself
uses. The gauge identity tuple,
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

## Static evidence for r2

r2 changes, all inside the diagnostic: the node masks and offsets above
replace the r1 guesses (`0x2000_0000` drawable, rect at `+0x38`), the walk skips
invisible subtrees like the walker does, each node prints its own, effective and
atlas rectangles, the live clip and the walker's draw context are copied, the
sample budget is 240 with change-driven triggers plus a periodic tick, and the
duplicate `tier` field plus the pointer-valued `first_sequence` left the line.

`./build.sh test` runs 10 host fixtures (`core/tests/behavior.rs`) then
`verify.py`. Fixtures: partial fill, empty, full, above maximum, depleted,
zero and negative maximum, extreme negative, the identity tuple against
`pl_gauge0` and stamina rows, the local-player chain (index 2, index 7, null
session, empty slot), signed gauge decode, a full controller sample carrying
150/160 at tier 2 with three drawables, a non-gauge controller costing no
sample, a gauge with no player, a tree past the visit limit, a hidden subtree
that must not be walked, and the fixture walk's node-by-node shape.
`verify.py` reports `PASSED`: 15 windows match the supplied NSO, the hook
window is unique (1 match), exports are exactly
`forge_onLoad/forge_onInit/forge_onUpdate/forge_onImGuiInit/forge_onImGuiRender/init/fini`,
every strong import resolves in main/sdk/forge, no settings or config import
exists, no `msr` in the disassembly, no `DT_TEXTREL`, and the payload is the
one NRO. `EVIDENCE.json` records the hashes.

## Run log read (Eden, r1, 2026-10-22)

The r1 candidate was installed in Eden's `load/0100770008DD8000/modern-sharpness-gauge/`
and the installed NRO hashed `9ebf36a31a23b0cca54e0e81762de1bee82620ece1780f0d64181c30ceaab92e`,
identical to the packaged payload, so the session ran these exact bytes. Forge
1.5.1 initialised; 63 of the 64 ERROR lines in `eden_log.txt` are ours (the other
is an Eden shader notice). The session ran t=0 to t=751 s.

What that run proved:

- the hook is reached and the identity gate works: 6001 walker calls with no
  match printed the absent-HUD line at t=54 (before any gauge exists), then the
  gauge controller matched at t=300 with `anim=4 objects=4 first_object=29
  sequences=15`, `ctrl=0x441838b0 row=0x53020238 owner=0x47f96900
  cache=0x480038e0 root=0x488279d0`, all stable across samples;
- the local-player chain resolved: `player=0x47e92570`, hunter index 0;
- the gauge words are points, signed: `current` 300 then 297 then 294,
  `maximum=300`, `tier=3`, `ratio_bp` 10000, 9900, 9800: three points per hit,
  tier unchanged in that window;
- the sample budget behaves as written: 60 samples between t=300.6 and t=418.7,
  then the "budget complete" line. The remaining ~330 s produced no lines, so
  whetstones, meals, Gunner sharpness, weapon changes, HUD recreation and the
  other hunter slots were never observed;
- the tree walk under-reported: `visited=3 visible=2 drawable=0` and every
  rectangle zero, because r1 tested `0x2000_0000` for drawable and never gated
  on visibility. r2 tests the mask the walker uses, skips invisible subtrees,
  and prints the own, effective and atlas rectangles plus the live clip;
- `first_sequence=1392641332` (`0x52E9E154`) showed row `+0x14` is a pointer,
  not the sequence row index Phase 0 recorded; r2 does not treat it as identity;
- the log holds no line that could change what is drawn, and nothing in it
  suggests a visual problem. The tester gave no separate note about the screen
  and no screenshot was recorded, so "nothing changed on screen" rests on the
  plugin's read-only design rather than on an observation.

Still open, and what the r2 run must answer: tier changes across colours, the
game's own sharpening paths (sheathe-sharpen plus whatever item the tester
confirms exists; this build has no long or short whetstone), any meal/skill/item
that raises sharpness or the maximum (MHGU has no Sharp Dashi meal; if this
build offers no sharpness buff, that negative result is what to report), separate
quests with different weapon types since MHGU carries one weapon per quest and
cannot switch mid-quest, death and timeout states, the hub before and after
quests and the map opened while running, hunter slots 1..3 online, the tip/hilt
direction of `Txt_sharp` from the printed rectangles, the object and drawable
counts and `cache_flags`. Whether a Gunner quest shows a gauge HUD at all is an
open question for the tester to report, not an assumption. And whether every
recovery path touches `+10100`/`+10104`: `adjustSharpness` at `main+0x2ad288`
is the only writer found, with gates at `0x27bcd0`, `0x27c7ec` and `0x27b918`
still unexercised. Still open for Phase 2: the `cGUI+0x2c` object map capacity,
the 16-byte cache growth per inserted object and the riding flag (r2 prints the
raw `input_flags` word at `+5156` for comparison and never interprets it).
riding flag stay for Phase 2: r2 only prints the raw `input_flags` word at
`+5156` for comparison, it does not interpret it.

## Phase 2 needs

The user's log from the r2 archive, matched against the NRO SHA-256
`acc9c2cf4b070c5c6ba6eecead82eb329e9ed7f1bb3430243bfcdc0b91262c61` in
`DISTRIBUTION.json` and `build.json`, recorded in `EVIDENCE.json` as runtime
evidence for r2. Phase 2 (drawing the gauge) starts once that log shows the
counts, rectangles and tier behaviour the design needs, and nothing in it
contradicts the r1 findings.

## Rules to keep

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/modern-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.
