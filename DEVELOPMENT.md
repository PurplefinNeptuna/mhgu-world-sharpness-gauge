# Developing modern-sharpness-gauge

Current state: Phase 2 candidate r17 (`DISTRIBUTION.json`, DEVELOPMENT, **not run**; nothing
installed by this workspace). r17 is the insertion build, and the first of this mod's versions
that changes what the game draws: one `cGUIObjScissorMask` under the gauge row's root, over the
bar, glow and edge, with the sharpness-ratio clip box written into the mask's built rectangle
from the draw walker. It carries the two storage expansions r14 lacked. The row's object array
at `instance+0xd8` (the same pointer the controller keeps at `+0xa0`) is grown from 4 words to 5
through the `cGUIInstance` DTI's MT allocator, `MtDti::getAllocator` at main+0x7aa47c, the way
Key Quest Markers r17 grows a row's map; the row's draw block is reallocated through the root's
own vtable slot 0x58 with main+0xa49b28's arithmetic, sized from the halfwords at `row+8`, with
the record pointer at `alloc + sequences * 12`. The r16 run of the read-only build (NRO
`415a822a2d3643efc1b5704c1e84814200be593c7922d297e85f535991acc0ed`, run on Eden 2026-10-23, bar
reported visible, 93 plugin lines, zero guest exceptions) is what the map and cache facts rest
on. r15 (NRO
`e90a6b54f2624c2cd6ae07b2567a14c794ebb7b34e068d841734e4541b18080f` — the exact installed bytes
are in EVIDENCE) was run on Eden on 2026-10-23, drew the gauge normally for 740 seconds and
closed clean; `work/modern-sharpness-gauge/runs/r15-eden-2026-10-23.txt`. r14 (NRO
`6e061c70779cfb3a306835030256012e50dd19fdbeeae36d4d4635c331d35b8d`) was run on Eden the same
day and the report is in `work/modern-sharpness-gauge/runs/r14-eden-2026-10-23.txt` and in
PHASE2_RESEARCH's r14 section: the gauge vanished, and the game crashed at the quest end.
Five facts, from those two logs and the decompiles in `work/modern-sharpness-gauge/
ghidra-batch10/`:
1. The clip never covered the bar. The rectangle the game built at `mask+0xb0..+0xbc` has
   left/top `0 0` in all 2,512 lines that printed it, while the requested box was
   `x=110 y=32` in 1,352 of them; the bar's own rect is `110,38,238,166`. The mask's position
   pair at `+0x80/+0x84` never reaches that build, so the mask clipped the gauge away from the
   insertion.
2. From t=642.30 the built rectangle is `0 0 0 0` for the last 1,119 lines while the plugin
   kept writing widths 18/33/49/80: the mask's authored size at `+0xa8/+0xac` is game-owned
   and was taken away at the row's layout change (the bar's rect became the authored
   `0,-6,88,18` on the same line, `cmd_tbl_id` 1000 → 9999 → 2003).
3. The crash is the row's object array indexed by the tree. main+0x5456e4 loops over the
   instance's array at `+0xd8` with `main+0xa38ffc`'s length = main+0xa47d2c's node count + 1.
   The gauge row has 4 objects for the root plus its 3 tree nodes; the mask made 4 nodes, so
   that pass walks 5 entries through a 4-entry array and calls vtable slot 0x50 on what it
   reads. The backtrace is that loop, R00 = `0x727272` (a freed block's `'r'` fill), and Eden
   died at t=1060.168. The row's object map, not the draw cache, is the thing an insertion has
   to grow.
4. The r15 run measured the rule on the unmodified row: `tree_nodes = 3` against `objects = 4`,
   with the gauge drawing normally for 740 seconds and no exception in the log. It also showed
   where the array lives: `+0xd8`/`+0xdc` read from the controller's owner field at `root+0xa0`
   are `0x727272`, a freed block's fill. The array is on the HUD instance the instance hook at
   main+0xa39300 records (its `+0xdc` is the gauge controller), which is the object
   main+0x5456e4 walks. r16 reads the facts from that captured instance and prints four array
   entries.
5. The cache sizing is main+0xa49b28's own arithmetic, read in full: the allocation is
   `(objects << 4 | 8) + sequences * 12` through the root's vtable slot 0x58 into `root+0xa4`,
   the walker's pointer `root+0x80` is `alloc + sequences * 12`, and the sizing pair is the
   halfwords at **row+8**. The r15/r16 runs measured that published offset at 192 = 16 * 12, so
   this row's sizing pair is 4 objects and 16 sequences, and its record area is
   `(4 << 4) | 8` = 72 bytes = 9 records. The tree with r14's mask emits exactly 9, so r14's
   records fit with zero headroom, and r14's crash is the map alone. Growing the map to 5
   objects makes the area 88 = 11 records.
6. r16 read the map. At `instance+0xd8`, the same pointer the controller keeps at `+0xa0`, the
   four entries are the controller, the bar, the glow and the edge: one word per object, length
   `tree_nodes + 1`, in its own allocation, outside the draw cache block. An inserted node needs
   a fifth entry. The accepted way to get one is Key Quest Markers r17's: allocate the new block
   through the owning class's DTI MT allocator (`MtDti::getAllocator`, main+0x7aa47c), copy the
   old entries, publish both fields, and free the old block through that allocator's free slot —
   the native destructor frees through the same allocator, and Forge's malloc cannot be
   substituted for game-owned GUI memory.
7. The mask's own enter handler main+0xa4ba94 says where the rectangle has to be written. While
   bit 1 of `mask+0xa4` is set it rebuilds `+0xb0..+0xbc` from the node's authored size pair
   `+0xa8/+0xac` laid out at the node's own position, which is how r14 got `0 0 W H` 2,512 times
   for a box that asked for `110,32`. It then reads those four words as integers, clamps them to
   the context's current box, and applies them while bit 0 is set. So the plugin writes the
   built rectangle itself, in the draw walker after the game's writes for the frame, keeps bit 1
   clear so nothing rebuilds it, and keeps bit 0 set to ask for the scissor.

r13 was reviewed before any run and is not installed. It linked the mask with the glow
and the edge left as siblings of the root, so the walk ended at the root without
emitting the mask's exit record: a scissor that never ends. It set the rectangle bit
and not the Enable bit, so no scissor was asked for at all. Its `apply()` re-read the
row with the gates written for the tree before the insertion, so it returned None after
the insertion. Its cleanup was a 3000-walker-call timer, which is not a liveness proof
and writes into a HUD the game may already have destroyed. All five are fixed in r14,
together with the "Every offset below is proven" header that was still in `gui.rs`.

Capture lifetime (plan step 3): r14 hooks the `cGUIObjRoot` destructor main+0xa486fc,
vtable slot 0x04 of main+0x178f9bc, which frees the row's cache and deletes the root's
child chain; the mask's own destructor main+0xa56758 deletes the chain below it, so the
game frees the mask and the three textures in the same pass. When the destructor's
argument is the captured controller, r14 drops the capture, the sampler's controller and
the insertion state before the original runs, and logs the fire. A session with no fire
line means this teardown path was never proven, and no cleanup claim is made for it.
`restore()`, `restore_links()` and the staleness timer are gone.

Run history: r5 stage 20, r6 stage 20, r7 stage 24, r8 stage 26, r9 stage 29, r10 stage 27 (107 lines: gates 25 and 26 passed, the parent word was read from the root instead of the bar), r11 stage 27 again (277 lines: the fix chained root+0x68+0x70, which is the root+0xd8 word, not the bar's parent) plus 112 stage-23 lines after a scene change on a capture whose row was gone. r8 and r9 both failed on gates copied from
Key Quest Markers whose model the logs refuted, so r10 follows playbook P3 step 4: every
gate is now written against the walker decompile, the `cGUIObjRoot` DTI record, and the
logged link words, not against another row's layout. r12 was packaged (NRO `01f06b9602fb57c2750e786efe757ca467a30483864a4faa3ac3c73198b5f22e`) and
never reported run; r13 was packaged and never run, and the review above is why. r14 was run
on Eden on 2026-10-23 with the installed bytes matching its candidate hash: the mask was
inserted, the gauge was clipped away from the insertion, and the game crashed at the quest end
with the emulator's own SIGSEGV. r15 removes the insertion and ran clean on Eden the same day
(740 seconds of gauge, 22 samples, no dynarmic exception, session closed normally); r16 fixes
which object the facts line reads the map from and ran once on Eden the same day: 93 plugin
lines, zero guest exceptions, the bar reported visible, and the map read out as controller, bar,
glow, edge at `instance+0xd8` with `tree_nodes = 3`. r17 puts the insertion back with both
expansions and has not been run.

User-reported play context (2026-10-23), recorded as report, not as runtime evidence of a
candidate: runs included the hub/base, and weapons with no sharpness (bowgun, prowler). The logs
match: stage 21 in bulk while no gauge exists, and a stage-23 streak after a scene change.
The live-gauge gate (stage 34, a sample with a positive maximum) stays in r14; the
staleness half of it is gone, because a call counter proves nothing about an object.

Open items: whether the mask clips at all is a screenshot question, and r14's log
prints both the box it asked for and the rectangle the game built at `mask+0xb0..+0xbc`,
next to the bar's rect, so a units offset shows up as a number. Whether reparenting
moved the bar is read from the tree lines before and after the insertion. Forge has no
plugin unload step, so nothing is undone at plugin removal: the game's own teardown of
the row is the only cleanup, which is what the destructor hook is for. Host fixtures
cover the tree before the insertion, the tree after it, the record counts and the
r13 shape; the ARM fixtures for insertion and allocation failure are not written.

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
| `0xa4ba20`, `0xa4ba40` | `cGUIObjScissorMask` Enable / Size setters | batch 3; Enable writes bit 0 of `+0xa4` (`bfc r2,#0,#1` + `bfi r2,r1,#0,#1`), Size stores `+0xa8`/`+0xac` |
| `0xa4844c`, `0xa48460` | native sibling / child linkers, each supplies the attached node's parent | instruction words in `PHASE2_RESEARCH.md`; no `bl` and no stored word for either address in main (`scripts/xref.py`, `scripts/find_word.py`) |
| `0xa4ba94` | mask enter record handler: builds the mask rect at `+0xb0..+0xbc` when `+0xa4 & 2`, applies the scissor when `+0xa4 & 1` | vftable word at `0x178fb3c` (= `cGUIObjScissorMask` slot 0x38), batch 7 |
| `0xa4bcdc` | mask exit record handler: restores the clip saved at `+0xc0..+0xcc`, returns unless `+0xa4 & 1` | vftable word at `0x178fb44` (slot 0x40), instruction words in `PHASE2_RESEARCH.md` |
| `0xa486fc` | `cGUIObjRoot` destructor (slot 0x04 of `0x178f9bc`): frees the row cache, then deletes the root's child chain. The r14 hook | instruction words, `find_word.py`; window signature |
| `0xa56758` | `cGUIObjScissorMask` destructor: deletes its own child chain, so the game frees the mask and the three textures together | instruction words in `PHASE2_RESEARCH.md` |
| `0xa4b6cc` | `cGUIObjScissorMask` factory: 0xd0 bytes through the mask DTI's MT allocator, `+0x5c` (sibling) zeroed | batch 7 decompile |

Runtime state read by the hook:

| Location | Meaning |
| --- | --- |
| `main+0x188ca48` | session singleton; `+0x30` is the local hunter index, valid 0..3 |
| `main+0x1885b58` | `sPlayer` singleton; `+0x1c + 4*index` is the local player pointer |
| player `+10100` | `uPlayerQuest::mSharpness`, signed points |
| player `+10104` | `uPlayerQuest::mMaxSharpness`, signed points |
| player `+5104` | `uPlayerBase::mSharpnessType`, unsigned tier index: 0 red, 1 orange, 2 yellow, 3 green, 4 blue, 5 white, 6 purple (mapping from the tester, 2026-10-22). A tier is a per-weapon segment of points |
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

## Static evidence for r4

r2 changes, all inside the diagnostic: the node masks and offsets above
replace the r1 guesses (`0x2000_0000` drawable, rect at `+0x38`), the walk skips
invisible subtrees like the walker does, each node prints its own, effective and
atlas rectangles, the live clip and the walker's draw context are copied, the
sample budget is 240 with change-driven triggers plus a periodic tick, and the
duplicate `tier` field plus the pointer-valued `first_sequence` left the line.

r3 changed only the sample trigger: the raw sharpness word is bucketed at 8
points before it can request a line (a hit costs a few points, so r2's raw-value
trigger would spend the whole budget inside one fight), a floor of 120 walker
calls separates change-driven lines, the player pointer joins the tracked set,
and the budget is 320 with a periodic tick every 2400 calls, so one run can
cover a session without the tester forcing anything.

r4 adds the `mCmdTblId` identity word and samples tier, maximum and player
changes at full resolution; 10 host fixtures cover the wider 116-word copy.

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

## Sharpness chain, read from the decompilations

`uPlayerQuest::adjustSharpness` (main+0x2ad288) is the only writer of
`mSharpness`. It applies the amount as a `short`, clamps the result to 0 and to
`mMaxSharpness`, recomputes the tier through `calculateSharpnessType`, compares
it with the `vft+0x1fc` accessor, stores `mSharpnessType` and fires event `0x1c`
(tier down), `0x1d` (tier up) or `0x1e` (reached the maximum) through `vft+0x3ac`.
That is the full write path, and it explains the measured r3 steps.

Skill effects on decay, all read in the same function: `HeavyPolish` zeroes the
decay and calls `vft+0x1b0` with `0x200`; `RazorSharp`, `StonefistSoul`,
`StonefistSoulX`, `SteadyHand`, `NightcloakSoul` and `NightcloakSoulX` halve the
decay, and a 1-point loss is nullified on a coin flip; `BluntEdge` doubles it.
This is why the r3 log showed costs from 2 to 42 points between samples: the
amount depends on the move and on the hunter's skills, not on a fixed per-hit
value.

The tier itself is weapon data, not player data:
`uPlayerBase::calculateSharpnessType` (main+0x29cdfc) takes the equipped record
through `FUN_00aa8814(&mpPlParam->mStatusParam, 0)`, resolves the weapon base data
with `FUN_00b0803c` over the singleton pair at `DAT_00c65ec0 + 0xc65e54`, asks that
object at `vft+0xd8` with `(equip->field1_0x4 & 0x3ff) >> 5`, and the returned
object answers points-to-tier at `vft+0x84`. So the per-weapon tier boundaries the
tester measured (250 points with a wide green, 200 with a thin green) live in the
weapon records, and one 5-bit field of the equip record selects the layout. The
DTI type record `/DTI_Types/WpDatEntry` (68 bytes) names the matching fields:
`Sharpness` at +21 and `SharpnessQuality` at +22, beside `WeaponId` at +58 and
`WeaponNameId` at +60. Those records are RomFS weapon data; this workspace holds
only `work/shared/game-exefs`, so reading the table for a named weapon needs a
RomFS extraction through the toolbox first (`docs/TOOLS.md`). Nothing in this
section is inferred from the game's numbers: each line names the function it came
from.

r4 adds the identity word `mCmdTblId` at player `+9388` (from the same DTI type
record) to every sample, so a line says which weapon produced it, and a tier,
maximum or player change is now sampled at full resolution without the rate
floor, so a boundary lands on the exact point.

## Phase 2, in progress (2026-10-22)

Phase 2 research lives in `work/modern-sharpness-gauge/PHASE2_RESEARCH.md`. Proven
there: the `cGUIObjScissorMask` field map (`+0x80/+0x84` float position with the
`0x20000` dirty bit, `+0xa4` mask flags, `+0xa8/+0xac` int size), the Size setter and
its `+0xa4 |= 2` dirty, the consumer at main+0xa4ba94 that only acts on that bit, the
mask push/pop in main+0xb375c4, and the scissor routine that writes the live clip at
`ctx+0xd0..+0xdc` which prepaint intersects. Geometry is decided in code: `boundary`,
`kept` and `prototype_ratio` in `core/src/lib.rs`, with the plan's ratios 0, 0.25, 0.5
and 1, hilt at the left, one shared screen column for the sword and its effect. Not
open in code: no mask object is created by this plugin yet, and none will be until the
constructor entry, the gauge row's map and cache publish contract and the ratio-zero
behaviour are proven. Four such items are listed in that note.

## Gauge scope, set by the tester on 2026-10-22

The bar length is `current / maximum` and nothing else. Tier boundaries, the
kireaji thresholds, the wide-thin point counts and the tier maths are the game's
own business: the mod does not compute a tier, does not pick a colour, and does
not change how the native gauge behaves. `mMaxSharpness` is read live, which is
why the level tables below are reference, not a requirement: the gauge never
needs them.

What the mod does need, and what the runs already proved:

- `mSharpness` at player `+10100` and `mMaxSharpness` at `+10104`, both signed
  points, read live: 250 and 200 on the tester's two weapons, depletion and
  whetstone recovery visible in the log.
- a ratio computed only from a verified positive maximum, clamped to `[0,1]`,
  with the native HUD untouched in every unsupported state.
- the native tier colour as the game produces it, read and reused, never
  re-derived from the ratio.

## Sharpness tables, read from the game's RomFS (Eden dump, reference only)

The RomFS is at `~/.local/share/eden/dump/0100770008DD8000/romfs/nativeNX/`, and
the sharpness layout the tier chain points at is in it.

`table/kireajiData.kad`: magic `0x3F99999A`, entry count 164, 14 bytes per entry
(8-byte header, then `164 * 14` = 2304 bytes, exact). Each entry is seven u16
cumulative thresholds. The rule that reproduces every tier transition measured in
the r3 log is: tier index = how many of the seven thresholds are strictly less
than the sharpness value. All 12 measured transitions across both weapons fit
that rule, and no type fits the non-strict reading.

`table/weapon07LevelData.w07d` (Long Sword, 876 entries, stride 15, header verified
by `8 + 876 * 15 == 13148`) and `table/weapon11LevelData.w11d` (Dual Blades, 845
entries, stride 17, `8 + 845 * 17 == 14373`) carry, per row: int32 weapon level
id, u8 weapon base id, u8 level, u8 kireaji type, u8 sharpness amount, int16 raw
damage. The maximum follows the modding wiki's formula
`150 + 25 * amount` (`references/MHGU-Modding.wiki/Weapon-Stat-Modding.md`), and
the r3 measurements match it: `maximum=250` is amount 4, `maximum=200` is amount 2.

Matching the r3 boundaries against the tables:

- Dual Blades, level 1, amount 2 (max 200), a boundary-consistent kireaji type:
  exactly one row, level id 147, base id 22, type 76, raw 80. Type 76 thresholds
  `80, 90, 190, 310, 390, 450, 450`, so at level 1 the segments are red 80,
  orange 10, yellow 100, green 190 to 200 = 10 points. That is the tester's thin
  green, from the game's own numbers.
- Long Sword, level 1, amount 4 (max 250), boundary-consistent types: two rows,
  both raw 130. Level id 55, base id 8, type 1 (thresholds `60, 90, 190, 290,
  390, 400, 450`: red 60, orange 30, yellow 100, green 190 to 250 = 60 points,
  the wide green). Level id 431, base id 61, type 65 (thresholds `60, 100, 190,
  310, 340, 420, 450`: red 60, orange 40, yellow 90, green 60). The r3 log cannot
  separate them: it needs one sample with sharpness between 90 and 99, where type
  1 reads yellow and type 65 reads orange.
- Types 62 and 111 also reproduce the log, and no level-1 row of the right amount
  uses them, so the level tables rule them out for these two weapons.

For Phase 2 this replaces guesswork with numbers: `mMaxSharpness` follows
`150 + 25 * amount`, the tier boundaries are the equipped weapon's kireaji
thresholds, and the fill fraction stays `current / mMaxSharpness` while the
colour comes from `mSharpnessType` computed from the same thresholds. A runtime
read of the kireaji type is the clean way to get boundaries for any weapon, and
it is reachable through the chain in the section above.

## Run log read (Eden, r3, 2026-10-22)

The installed NRO hashes `c35bc25afbe4ed72ed91b20639aa35bef1a3156b3b294b14292f8430e4c5f47d`,
equal to the r3 packaged payload, so this session ran the recorded bytes. 238 of
our lines: 1 absent, 1 first-observed, 59 samples, 177 tree lines, over a 3140 s
session. The bucketed trigger behaved: 59 of 320 samples covered t=204 to t=2483,
where r1 spent 60 samples in 118 s.

Measured, with the tester's three hunts:

Both hunts used weapons whose top tier is 3 (green), which is why no blue,
white or purple line appears in the log. With the tester's colour mapping:

- Long Sword, `maximum=250`, top tier green: green 250 down to 196, yellow 189 to
  115, orange 82 to 62, red at 57; a whetstone moved 57 to 155 (yellow).
- Dual Blades, `maximum=200`, top tier green: green at 200 down to 191, yellow
  from 190, red at 77, orange at 88 in a later cycle; whetstones restored to 200
  twice.
- The tester's own description of these weapons, a wide green tier on the Long
  Sword and a thin green tier on Dual Blades, matches the measured point counts
  (250 versus 200 points in the same 128x128 box), so wide and thin is about the
  stats, not the drawn size.
- Light Bowgun: no identity match, no sample and no tree line for the whole
  window after t=2483, and the tester saw no sharpness icon. The gauge HUD is
  absent for that weapon type, and the mod reads nothing there.
- Tier boundaries are per-weapon point segments, not percentages: 250 points
  crossed a tier at 189 (75.6%), 200 points crossed at 190 (95%). The fill length
  must come from `current/max` and the colour from `mSharpnessType`; a ratio can
  never be used to pick the tier, and the tester's own note is that every weapon
  draws the gauge the same size while the point counts differ.
- Both weapons use the same 128x128 box: own rect `110,38,238,166` and
  `116,32,244,160`. The gauge is drawn by two visible sprites plus one drawable
  sprite with the visibility bit clear (the `Txt_sharpEdge` case from Phase 0),
  walked as 3 nodes with 2 visible and 3 drawable.
- The native gauge is frame based: the atlas rectangle at node `+0x10C` is four
  floats and it steps `0,280,88,304` to `0,304,88,328` to `0,328,88,352` with
  sharpness, beside a companion `0,160,88,192` glyph. Sharpness is shown by
  picking atlas rows, not by a continuous fill, so a continuous fill needs its
  own geometry and mod-owned resources.
- Clip and effective rectangles are all zero: `context+0xD0..0xDC` reads 0,0,0,0
  and node `+0xF0..+0xFC` reads zero, so no scissor was applied to the gauge. The
  16-byte cache growth and the `cGUI+0x2c` map question stay for Phase 2.
- `cache_flags` read 0 at every sample. Controller, root and player pointers all
  changed between the two quests, which is the recreated-HUD path, and hunter
  index stayed 0 because the online part was not played.
- Field corrections from these bytes: the four words at `+0x10C` are floats, so
  `1134034944` is `308.0` (setTextureRect writes them through a `float *`), and
  the word at `+0x33` has low nibble 0 on all six gauge nodes, so it is not the
  kind word prepaint tests. The observed flag words are `0x02800005` (visible,
  drawable, bit 23) and `0x02040000` (drawable, bit 22, visibility clear).

## Phase 2 static evidence (r5, r6)

`work/modern-sharpness-gauge/PHASE2_RESEARCH.md` holds the full record; the parts
this mod depends on:

- The mask factory is `main+0xa4b6cc`. It takes no arguments, allocates 0xd0 bytes
  through the mask DTI's own MT allocator (`MtDti::getAllocator` at `main+0x7aef68`,
  slot +0x20, size 0xd0, align 0x10, id from the DTI record), stores the vtable,
  writes the mask defaults (128x128 at +0xa8/+0xac, +0xa4 |= 3, +0x8c = 1.0f) and
  walks the parent chain on `+0x70` comparing `+4`. It is reached through the class
  table at `main+0x178ff04`; no `bl` targets it.
- `cGUIObjScissorMask::DTI` is at `main+0x21233e8`: size 208, vtable
  `main+0x178fb04`, id 0xda93f7a, parent `cGUIObj2D`, `mId` at +0x50. DTI
  `get`/`set` and `vft` numbers are main-relative, the same convention as every
  other offset in this mod; Ghidra's function names are 0x9c9000 higher, and the
  export files are named by main-relative offset.
- The instance is not derivable from the controller. `main+0xa39300` is the only
  caller of the draw walker in all of .text (one `bl`), and it reads the controller
  at `instance+0xdc`. Second hook, same relocation rules: prologue
  `push {r4-r11,lr}` plus `add r11, sp, #0x1c`, unique 12-word window.
- The row's map is `[controller, tree root, ...]` and the gauge's tree root is
  `Txt_sharp`: the r5 log prints `root` equal to node n0's address. So the group
  `sharp` is the bar's parent above the walker's entry point, derived from
  `bar+0x70`, never indexed from the map.
- The bar is identified by its authored size 88x24 read from the live texture
  fields +0xb4/+0xb8, cross-checked against the bar -> effect -> edge chain in both
  directions. In `hud0.gui` the bar's ink occupies columns 18..=79 and rows +6..+17
  of that rect, measured from the decoded atlas, so the clip track starts 18 in from
  the rect's left and runs 62 columns. Ratio 0 asks for an 18-wide clip, so the
  degenerate zero-size mask is never used.

## r5 runtime reading and the two fixes in r6

The tester's r5 run on Eden sampled the gauge correctly (Long Sword 300/300 down to
260/300, tier 3, three walked nodes) and inserted nothing, reporting
`mask insertion failed at stage 20` at the first update, 16 seconds after launch.
Two defects, both fixed in r6:

1. Stage 20 means the row is not present yet, which is normal before the gauge HUD
   exists. r5 marked it terminal. r6 reports it once and retries every 240
   attempts; only factory, allocation and publish failures are terminal.
2. r5 recorded the last instance that called the walker. Every HUD animation
   instance calls it, so the pair was a race. r6 records only a pair whose
   `instance+0xdc` is the gauge controller the sampler identified.

`restore()` has no caller: the vendored forge-rs exposes no plugin unload callback,
so the mask lives until the process exits. Recorded as a gap, not hidden.

## Phase 2 needs

The user's log from the r2 archive, matched against the NRO SHA-256
`63b69bc93edf3dec646a7028e3bfaebf41b017b166ca827a28ba01447bf62992` (r4) in
`DISTRIBUTION.json` and `build.json`, recorded in `EVIDENCE.json` as runtime
evidence for r2. Phase 2 (drawing the gauge) starts once that log shows the
counts, rectangles and tier behaviour the design needs, and nothing in it
contradicts the r1 findings.

## r15, built 2026-10-23: the insertion is gone

NRO `e90a6b54f2624c2cd6ae07b2567a14c794ebb7b34e068d841734e4541b18080f`, run on Eden on
2026-10-23 with the installed bytes matching (EVIDENCE's r15 run record; log
`work/modern-sharpness-gauge/runs/r15-eden-2026-10-23.txt`). The one change: r15 creates no GUI
object and writes no game memory. It
keeps the three hooks (walker main+0xa494a0, instance draw entry main+0xa39300, root
destructor main+0xa486fc), the sampler, and gates 21..34, and prints one
`MSG r15 gauge row facts` line per gauge row: the controller's root/cache/flags/row/owner and
cache-allocation words, the row's anim/counts/first_object/+0x14, the instance's object-array
pointer and its first three entries, each texture's mId at +0x50 and authored size at
+0xb4/+0xb8, the bar's flag word, and `tree_nodes` = main+0xa47d2c's count for this
controller. The line answers one question in numbers: `tree_nodes + 1` against the row's
object count, which is what main+0x5456e4 indexes its array with, and what r14 broke.

Host fixtures: 19, adding `the_object_array_is_sized_for_the_tree_it_was_built_with` (3 tree
nodes + 1 = this row's 4 objects; r14's shape is 4 nodes, so that pass indexes 5 entries
through a 4-entry array). `core` gained `count_nodes`, the port of main+0xa47d2c.

Deleted with the insertion: the factory call and the four link writes in `gui.rs`,
`apply`/`rect`/`link`, the per-second clip write and the mask-rect log lines, the elapsed-time
counter, and `memory::write_word`/`write_bits`/`call0`/`call2`. The mask geometry helpers in
`core` stay: they are measured from hud0, tested, and the next step reuses them.

What r15 does not prove: any drawing. The root-destructor hook has zero fires to its name in
the only session that logged it, so the teardown path stays unproven and no cleanup is claimed.

## r15 run 2026-10-23 (Eden): clean, and two answers

658 log lines, sha256 `c6cffe82380518e6b77368a1179abe0b07ffcaee4e5bc7e7c5a015bc5f9c909f`. 124
plugin lines: 33 stage-21 lines while the HUD was absent (first at t=29.76, the hub), one
"no sharpness gauge controller seen in 6004 walker calls" at t=55.83, the gauge controller seen
at t=159, 22 samples (Long Sword 250/250 tier 3 → 126/250 tier 2 → 226 tier 3), 66 tree lines
with 3 nodes, one facts line at t=163.53, zero teardown lines, zero dynarmic exceptions, and
the session closed normally at t=903. The gauge row read the whole time: controller 0x4800b250,
row 0x519a0238, anim 4, objects 4, sequences 15, first_object 29, `cache_flags` 0 → 1.

Two answers, and one thing the build got wrong. `tree_nodes = 3` against `objects = 4` is the
unmodified row, exactly the size main+0x5456e4 indexes, so the rule from the r14 crash holds on
live data. The published cache sits 192 bytes into the allocation
(`cache_alloc=0x48023820`, `cache=0x480238e0`). main+0xa49b28's arithmetic, read in full later,
fixes that as this row's sequence prefix of 16 * 12, so the sizing pair is at `row+8` (4
objects, 16 sequences) and the record area is `(4 << 4) | 8` = 72 bytes = 9 records: r14's
9-record tree fit exactly, with no headroom. Pairing a 252-byte size read off the `row+4`
counts with this 192-byte prefix gave 60 bytes and 7 records, and that reading is refuted: the
sizer uses one field, the `row+8` pair, for both numbers. The map fields came out as
`obj_list=0x727272`, `inst_ctrl=7500402` (= 0x727272), because facts read them from
`root+0xa0`, the controller's owner field. That object does not carry the array; the instance
the instance hook records does.

## r16, built 2026-10-23: the facts read from the captured instance

NRO `415a822a2d3643efc1b5704c1e84814200be593c7922d297e85f535991acc0ed`, run once on Eden on
2026-10-23 with the installed bytes matching, the user reporting the bar visible; nothing
installed by this workspace. Two changes, both in the facts line. It now takes the HUD instance from
`hooks::gauge_instance()` (the pair the instance hook records at main+0xa39300, where
`instance+0xdc` is the gauge controller) instead of `root+0xa0`, and it prints four entries of
that instance's array at `+0xd8`, the length main+0x5456e4 indexes it with. `FACTS_WORDS` is 28;
the field named `owner` is now `instance`. 19 host fixtures pass, verify passes, and nothing
else in the build moved: same three hooks, same gates 21..34, same sampler, no writes.

What r16 asks the run for: the four array entries (the map's contents, which is what the next
insertion step needs) and `tree_nodes` beside them, plus whether the bar is visible through a
whole quest and whether the results screen appears.

## r16 run 2026-10-23 (Eden): the map's contents

634 log lines, sha256 `cfea8b166128bce29b4bded10c2430c9ca3b87472d393ed10bcfeb81d5edebf0`, 93
plugin lines, zero guest exceptions, installed bytes matching the r16 payload. 63 stage-21 lines
in the hub, gauge controller seen t=159.19, 7 samples, 21 tree lines with 3 nodes, one facts line
at t=163.5, zero teardown lines, last plugin line t=525.92. The user's report: the bar was
visible. Eden's own ending is in the log (three `maxwell_3d.cpp:392` Critical asserts at
t=658.73, `ForceStop` at t=670.55, host `Unhandled SIGSEGV at 0x00007fdde549b8d4` at t=670.97,
with no guest exception), and the r15 log of the same day has the same ForceStop and 15 Critical
records without the SIGSEGV.

The facts line: instance 0x47fdd180, `instance+0xdc` = 0x47fceec0 = the gauge controller, array
at `instance+0xd8` = 0x47f959b0, entries `[0x47fceec0, 0x48008230, 0x47ff7560, 0x47fdda80]` =
controller, bar, glow, edge, `tree_nodes = 3`. The array is one word per object and its length is
`tree_nodes + 1`, exactly the number main+0x5456e4 indexes with. The controller's `+0xa0` field
is the same 0x47f959b0 pointer, so r15's `0x727272` was reading `+0xd8` through the array pointer
itself, 216 bytes into a 16-byte array. `cache_alloc` is 0x48013ab0, far from 0x47f959b0: the
array is its own allocation, and its allocator/freeer are unresearched. That is the next static
question, and it decides how a fifth entry can exist.

## r17, built 2026-10-24: the insertion, with both storage expansions

`src/insert.rs` is the new module; the read path (gates 21..34, the sampler, the tree log) is
r16's. One insertion per gauge row, from `gui::tick()` right after the facts line, inside the
walker's callback where the controller is reachable in the same call. Order, and why:

1. Read the row and the row's **sizing pair at `row+8`** (`objects`, `sequences`) — the fields
   main+0xa49b28 uses, not the `row+4` pair the facts line prints as `counts`; r17 prints that
   pair too, as `counts8`. `objects` must equal `count_nodes(root) + 1` (3 nodes, 4 objects on
   the unmodified row) or the insertion stops at stage 48: that equality is the r14 crash rule,
   read before touching anything.
2. Build the mask with the class's own factory main+0xa4b6cc, which allocates the 0xd0-byte
   object through the mask DTI's MT allocator. Typed against the `cGUIObjScissorMask` DTI
   (vtable slot 0x14), then: `+0x50` = marker `MSG1` (the factory leaves -1 there, so a later
   pass finds an existing mask instead of inserting a second), `+0x74`/`+0x78` = the bar's
   owning cGUI and borrowed resource read live, `+0xa4` = 1 (enable, no rebuild).
3. Grow the map (`grow_map`): allocate `(entries + 1) * 4` bytes, 16-aligned, through the
   `cGUIInstance` allocator (vtable slot 0x1c), copy the original 4 words at their original
   indices, append the mask, publish `instance+0xd8` **and** `root+0xa0` (the r16 run showed the
   two fields hold the same pointer; main+0xa49b28 writes `+0xa0` from its third argument), then
   free the old block with the allocator's slot 0x34. Stages 42/43.
4. Grow the draw block (`grow_cache`) with `cache_layout` = `(objects << 4 | 8) + sequences * 12`
   for `objects + 1`, allocated through the root's vtable slot 0x58 into `root+0xa4`, zeroed,
   `root+0x80` = `alloc + sequences * 12`, then the old allocation freed through slot 0x5c.
   Stages 44..46. The root allocated the block it frees, which is the contract Key Quest Markers
   uses for the same pair.
5. Link last, so a 5-node tree never exists against the old 4-word array: native linkers
   main+0xa48460 (parent, child) and main+0xa4844c (node, sibling, which also sets the sibling's
   parent), root → mask → bar, bar → glow → edge. Then `root+0x8c |= 3` asks the walker to
   rebuild the cache, the same request Key Quest Markers makes. Stage 47; the first link call is
   the only write that can land partially, and it is reverted by re-linking the root to its
   original child.
6. Per frame, in the walker hook for the gauge controller only: `insert::apply` computes the box
   from the bar's rect and the live ratio (`mask_box` and `boundary`, the core's own rules) and
   writes `+0xb0..+0xbc` as four integers, the authored pair `+0xa8/+0xac`, the position pair, and
   `+0xa4` = 1. Nothing is written while the box is unchanged. The gates accept the inserted
   tree: `read_row` detects the marker at the head of the chain and checks the three textures
   against the mask as their parent, which is what the r13 review called out.

Failure codes 40..49 are the insertion's own, kept apart from the row gates 21..34; each is one
log line naming the step. `scissor mask inserted` reports the mask, map, entry count, cache
allocation and record capacity, and `clip mask rect` prints the rectangle that is in the object
right now next to the bar's rect every ~2 seconds — the readback that answers the r14 question.
Teardown: the root-destructor hook clears the mask, bar and box addresses (it has fired zero
times in the three logged sessions, so no cleanup is claimed).

Host fixtures: 20 pass, the two new ones being the cache sizing against the r16-measured 192-byte
prefix (72-byte area, 9 records, so r14's tree fit exactly and the r15 section's 60-byte reading
is refuted) and the node-count rule on both trees.

## Rules to keep

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/modern-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.
