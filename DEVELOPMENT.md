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

## Phase 2 needs

The user's log from the r2 archive, matched against the NRO SHA-256
`63b69bc93edf3dec646a7028e3bfaebf41b017b166ca827a28ba01447bf62992` (r4) in
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
