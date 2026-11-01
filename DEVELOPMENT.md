# Modern Sharpness Gauge: development

**Current state.** r63, READY_TO_DISTRIBUTE, the mod as it ships: the accepted cut and the accepted shade.
The shade's atlas window is the bar's own live four floats - `u` = `0..88` of the authored 88, `v` = the
bar's current tier band - published through the `mTextureRect` setter every frame. The shade is the clip
mask's sibling, one clip mask with the copy as its child, `#202326` = `0xFF262320`, the clip mask carrying
bit 17, the copy carrying the bar's sampler. 6 tree nodes, the 7-word object array, `(6 << 4) | 8` = 104
bytes = 13 records, and the walk emits exactly 13. 23 host fixtures pass; verify PASSED with 33 signature
windows; distribution check PASS.

r63 itself has never been run: it is r62 with the run instruments removed, promoted on static validation
by the user's choice. Runtime evidence is r62, accepted on Eden on 2026-11-01 at the red tier ("looks
correct now, please pin this version as candidate for release"), frozen in
`backups/accepted/modern-sharpness-gauge/r62-screen-accepted/`. Payload NRO
`202c0719c7edfb28f4b3b359630b60f84ec69ac00c632a573284f7b2d3cc965f`. `DISTRIBUTION.json` holds the status
and hashes, `EVIDENCE.json` the exact results, `TESTING.md` the tester steps and log lines.

The window is published through the setter because the drawn quad's UVs are a cache: the texture's draw
slot main+0xa4fc9c calls main+0xa4feb4, which reads the UVs from `+0x10c..+0x118` and the four corner
colours from `+0xa4` only while `(global+0x98 & (node+0xc0 >> 24) & 7)` is nonzero and clears those bits
afterwards, and the setter main+0xa4fb58 - early return while the four floats match, store, slot 0x28 when
`+0x108 & 0x30`, tail-call of vtable slot 0x50 (main+0xa4fc68) - is the only path to that mark. A raw
write to the copy's `+0x10c` marks nothing and never repaints. `work/modern-sharpness-gauge/PHASE5_RESEARCH.md`
carries these facts with their bytes.

General GUI rules (masks, cache and map growth, teardown) are in the forge-modding skill's
`references/native-gui-extensions.md`; row, field and table facts are in the mhgu-understanding skill's
`references/sharpness.md`.

## Constraints

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/modern-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.

## Commands

`./build.sh doctor|build|test|verify|package|export --output DEST`. There is no
`prepare`: this mod ships no resources, assets or INI, and has no settings,
so no Plugin UI panel: the build exports `forge_onLoad`, `forge_onInit`,
`forge_onUpdate`, `init` and `fini`, and `verify.py` requires exactly that set.
Inputs come from `build.local.json` (`main`, `sdk`, `forge` under `inputs`,
copied from `work/shared/game-exefs/`); `build.local.example.json` documents
the keys. Outputs go to `build/modern-sharpness-gauge/candidates/VERSION/` and
scratch to `work/modern-sharpness-gauge/candidates/VERSION/`, where VERSION is
`scripts/paths.py`'s current version. `package` refuses to run for a
`READY_TO_DISTRIBUTE` spec: no distribution approval exists.

r63's source comments were corrected on 2026-11-01, after its release, with no
line added or moved. A scratch rebuild of the corrected source gives the released
NRO `202c0719c7edfb28f4b3b359630b60f84ec69ac00c632a573284f7b2d3cc965f`, so the bytes
are unchanged, while the `source_sha256` values in r63's `build.json` still name the
source as it stood at the build.

## Files

- `core/src/lib.rs`: `#![no_std]` decisions only: the local-player chain, the
  gauge read, the fill math in basis points, the gauge-HUD identity, the cut
  box from the accepted rect offsets, the node count and cache sizing the
  insertion gates on, the walker ports the fixtures pin, and the copied `Sample`
  with its 11 field names. No Forge, no FFI, no `unsafe`.
- `core/tests/behavior.rs`: 23 host fixtures over a fake memory map.
- `src/hooks.rs`: the four Forge hooks (walker, instance draw, row root
  destructor, mask enter), their signature gate and rollback.
- `src/sampling.rs`: discovery in the walker's hot path: the slow attempt
  cadence, the captured controller and player, the gauge-present gate, and the
  capture's teardown.
- `src/gui.rs`: the row gates 21..34 and the retry cadence.
- `src/insert.rs`: the mask, clip mask and copy, the map and cache growth, the
  per-frame writes at the mask's enter handler, and the liveness checks.
- `src/memory.rs`, `src/svc.S`: mapping-checked reads and the three SVC
  wrappers Forge does not expose (query memory, GetInfo program id, process
  cache flush). Gap documented beside each use.
- `src/signatures.rs`: 33 32-byte windows taken from the original main NSO.
- `src/lib.rs`: metadata record, install, and the log lines outside the hooks.

## Offsets, units and proof

All values are main-module relative. Ghidra's base is `0x009c9000`; subtract it
for addresses from Ghidra, never for vtable words or DTI `get`/`set` values, which
are already main-relative. Disassembly prints words byte-swapped: the word
`e92d4830` is the memory bytes `30 48 2d e9`, so a 12-word window is written in
memory order as `30482de9 0140a0e1 ...`. `scripts/verify.py` compares memory-order
bytes against the supplied NSO; nothing here is a Ghidra-imported byte.

| Address | Meaning | Proof |
| --- | --- | --- |
| `0xa494a0` | draw walker entry; a hook site | 12-word window, unique in main; `bl` from `0xa39664`, tail branch from `0xa56a20` (`research xref`) |
| `0xa49b28` | animation controller setup; sizes the draw cache from the halfwords at row+8 | decompile in `ghidra-batch1/` |
| `0xa39300` | instance draw; hands the controller to the walker at `0xa39664`; a hook site | same batch |
| `0xa37c1c` | scissor routine: writes the render target's scissor and stores the request at context `+0xd0..+0xdc` | batch 1 |
| `0xa4e628` | `cGUIObjTexture` slot 0x38, its enter: unions the live scissor with its rect only when `+0xcc`'s low nibble is 3 or 6 and `+0x56` bit 6 is clear | vtable word, bytes |
| `0xa4fc9c`, `0xa4feb4` | texture draw slot 0x3c; rebuilds the quad's UVs and colours only on the `+0xc0` mark | batch 1, bytes |
| `0xa4fc68` | texture slot 0x50: sets the `+0xc0` bits 24..26 | bytes |
| `0xa4964c`, `0xa4915c` | transform walk (slot 0x44 only with bit 17); row update with the bit 19 child re-sort on `+0x60` | bytes |
| `0x61912c` | HUD widget driver, gauge case 8 reads the tier | batch 2 |
| `0x26893c` | `uPlayerBase::getSharpnessType`, returns `mSharpnessType` | batch 2 |
| `0x29cdfc` | `uPlayerBase::calculateSharpnessType` | batch 2 |
| `0x2ad288` | `uPlayerQuest::adjustSharpness`, the writer of `mSharpness` | batch 2 |
| `0x3f8398` | local hunter index reader over the session singleton | batch 3 |
| `0x276278` | `sPlayer::getMainPlayer` | batch 3 |
| `0xa4faf0`, `0xa4fb58` | `cGUIObjTexture` setTexture / setTextureRect | batch 3, bytes |
| `0xa4ba20`, `0xa4ba40` | `cGUIObjScissorMask` Enable (bit 0 of `+0xa4` from its argument) / Size setters | bytes |
| `0xa48460`, `0xa4844c` | native child / sibling linkers, each writes the attached node's parent | bytes; `bl` from `0xa38dd8` and `0xa38dfc` |
| `0xa4ba94` | mask enter: builds `+0xb0..+0xbc` when `+0xa4 & 2`, applies the scissor when `+0xa4 & 1`; a hook site | vtable word at `0x178fb3c`, batch 7 |
| `0xa4bcdc` | mask exit: restores the clip saved at `+0xc0..+0xcc`, returns unless `+0xa4 & 1` | vtable word at `0x178fb44`, bytes |
| `0xa486fc` | `cGUIObjRoot` destructor (slot 0x04): frees the row cache, then deletes the root's child chain; a hook site that has fired in no saved log | vtable word, bytes |
| `0xa56758` | `cGUIObjScissorMask` destructor (slot 0x04): deletes its own child chain | vtable word, bytes |
| `0xa4b6cc` | `cGUIObjScissorMask` factory: 0xd0 bytes through the mask DTI's MT allocator | batch 7 decompile |
| `0xa38828` | `setAnimationId`: destroys a row's controller and map, builds new ones; `bl` from `0x54504c` and `0x5451fc` | batch 1 |

Runtime state read by the hook:

| Location | Meaning |
| --- | --- |
| `main+0x188ca48` | session singleton; `+0x30` is the local hunter index, valid 0..3 |
| `main+0x1885b58` | `sPlayer` singleton; `+0x1c + 4*index` is the local player pointer |
| player `+10100` | `uPlayerQuest::mSharpness`, signed points |
| player `+10104` | `uPlayerQuest::mMaxSharpness`, signed points |
| player `+5104` | `uPlayerBase::mSharpnessType`, unsigned tier index: 0 red, 1 orange, 2 yellow, 3 green, 4 blue, 5 white, 6 purple (mapping from the tester, 2026-10-22). A tier is a per-weapon segment of points |
| controller `+0x68/+0x80/+0x8c/+0x9c/+0xa0` | tree root, draw cache, rebuild flags, animation row, map alias |
| row `+0/+4/+0xc/+0x14` | animation id; counts packed in one word (objects low u16, sequences high u16); first object row; the word at `+0x14` is a pointer into the sequence block, not a row index |
| node `+0x54` | flag word: `.agents/skills/mhgu-understanding/references/native-gui.md` has the bits |
| node `+0x68/+0x6c/+0x70` | first child, next sibling, parent |
| node `+0xE0..+0xEC` / `+0xF0..+0xFC` | layout rect, 128x128 here and not the sprite (4 signed ints), then the effective rect |
| node `+0x100/+0x104/+0x10C..+0x118` | texture id, texture object, atlas rect (4 floats) |
| context `+0xD0..+0xDC` | the live clip |

The gauge identity tuple, identical in every language and in HD: animation id 4,
4 objects, first object row 29, 15 sequences. Stamina and `pl_gauge0` rows differ,
so the identity test is the cheap filter in front of every read. The singletons live
in `.bss`, past the NSO file image, which is why `verify.py` checks the `.data/.bss`
split instead of their bytes; the chain itself is proven from the decompilations
above and is read through the mapping query at runtime.

## Hot path rules

- Four hooks, each `e51ff004` + target. Forge copies the first 8 bytes without
  relocating them, so each entry is checked for position independence; install
  validates the 16-byte continuation (original bytes, branch opcode, `target+8`)
  and rolls back on any mismatch. Hook struct size 24, `context` at 12, `ctx_cell`
  at 16 (compile-time asserts).
- No `MSR` anywhere in the plugin (`verify.py` disassembly gate). The walker's
  relocated window keeps `cmp r1,#0` with the flag-consuming `popeq`, and holds no
  SEL/USAD8/MRS/MSR, so no `APSR_g`/`APSR_nzcvqg` producer is separated from
  its consumer.
- Inside the hooks: no allocation, formatting, file access or logging. Reads go
  through `memory::word`, which queries the mapping per access.
- Discovery cadence: one attempt every 60 walker calls, so a few fractions of a
  second per HUD. Nothing is queued, and `FIELDS` names the copied words;
  formatting and logging happen only in `forge_onUpdate`.
- Fill math, in `core`: require a live player and a maximum `> 0`, compute in
  `i64` basis points, clamp to 0..=10000. A zero, negative, missing or
  above-maximum state yields no ratio; above maximum draws full, depletion
  floors at empty. Nothing is ever divided by an unverified maximum.

## Accepted shape

- **Scope, set by the tester.** The bar's length is `current / maximum` and nothing else. The mod computes
  no tier and picks no colour; the tier colour stays the game's. The native HUD is untouched in every
  unsupported state.
- **Tree.** The controller's first child is the mask over bar, glow and edge; the clip mask is the mask's
  sibling, with the copy as its child. The walk emits mask enter, bar/glow/edge enter+exit, mask exit, clip
  enter, copy enter/exit, clip exit, terminator, so the copy is outside the accepted scissor and inside its
  own. Teardown follows the same links: the mask destroys bar, glow and edge, and its sibling chain destroys
  the clip mask, which destroys the copy.
- **Cut.** Offsets 18 and 90 of the bar's 128 px rect, scaled by the live rect width, linear between them.
  On the measured 110..238 rect the box rights are 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%.
  These are the tester's measurement (128 at 0%, 200 at 100%), not derived from the atlas. At the accepted
  cut, 0% leaves only the hilt glyph and 100% ends at the measured tip.
- **Measured art.** Ink in hud00 rows 280, 304 and 328 is columns 18..79 and rows +6..+17 of the authored 88x24:
  hilt 18..27, blade 28..79. On screen the ink runs 124..200 px and the hilt glyph starts near 125. On the crops
  1.5 image px is 1 GUI px. These describe the art; the clip's numbers come from the photographs. Hilt columns:
  18-19 left edge, 20-21 empty, 22-25 hook, 26-27 empty; the tip tapers at 76..79. The glow's bloom reads 3..9 px
  past the box in a photo, which is not a misplaced cut. Screenshot measurements carry a few px of noise.
- **Shade.** Each frame the clip mask's translation (`+0x40/+0x44`) is the cut's right edge and the bar's
  rect top; its authored width is the blade past the cut and its height the bar's rect height. r62 logged
  `shade_box=128 38 200 166` at `ratio_bp=0`. The copy keeps the bar's quad from the build, and only its
  window changes per frame. The dark part has no glow by design.
- **Copy construction.** DTI allocator slot 0x20 with `(bytes, align, dti id)`, a zeroed 304-byte block,
  constructor main+0xa4f60c, the mod's marker at `+0x50`, the bar's `+0x64/+0x74/+0x78`, the twelve style setters in Key Quest
  Markers' order, `SamplerState` from the bar (authored 2, stored in the low nibble of `+0x108` by
  main+0xa4fa5c), the `Texture` id last, then the window through the setter. Stages 50..52 are additive: no
  copy, and the gauge looks like the plain cut.
- **Transform.** Bit 17 of `+0x54` is transform dirty. The walk main+0xa4964c, started by main+0xa39c40 when
  root `+0x8c` bit 1 is set, calls setMatrix (slot 0x44, main+0xa4e604 for a texture) only on a node with
  that bit, and pushes the bits to children only from a dirty node. Mask defaults come from factory
  main+0xa4b6cc, which is reached through the class table at main+0x178ff04.
- **Lifecycle.** A gauge row dies at every area's loading screen. The walker hook checks the marker and links
  read-only; on failure it runs `hooks::teardown_gauge_row`, so the next row is read and inserted like the
  first. The mask's enter disables a mask whose links are gone. Forge has no plugin unload, so nothing is undone at removal.

## Sharpness chain

`adjustSharpness` (main+0x2ad288) fires event `0x1c` (tier down), `0x1d` (tier up) or `0x1e` (reached the
maximum) through `vft+0x3ac`. Decay, read in the same function: `HeavyPolish` zeroes it and calls `vft+0x1b0`
with `0x200`; `RazorSharp`, `StonefistSoul`, `StonefistSoulX`, `SteadyHand`, `NightcloakSoul` and
`NightcloakSoulX` halve it, with a 1-point loss nullified on a coin flip; `BluntEdge` doubles it. Gates at
`0x27bcd0`, `0x27c7ec` and `0x27b918` are unexercised.

The tier is weapon data. `calculateSharpnessType` (main+0x29cdfc) takes the equipped record through
`FUN_00aa8814(&mpPlParam->mStatusParam, 0)`, resolves the base data with `FUN_00b0803c` over the singleton
pair at `DAT_00c65ec0 + 0xc65e54`, asks it at `vft+0xd8` with `(equip->field1_0x4 & 0x3ff) >> 5`, and the
result answers points-to-tier at `vft+0x84`. `/DTI_Types/WpDatEntry` (68 bytes) names `Sharpness` +21,
`SharpnessQuality` +22, `WeaponId` +58 and `WeaponNameId` +60. `mCmdTblId` at player `+9388` identifies the weapon: 1000 Long Sword, 2000..2033 Dual Blades, 9999 no weapon command (bowgun).

## Do not

- **Leave the glow and edge outside the mask, or set only the rebuild bit.** A mask whose tree never closes has no exit record, so its scissor never ends; `a_mask_left_open_never_stops_clipping` pins that shape. `+0xa4` bit 1 only rebuilds, and bit 0 asks for the scissor.
- **Publish a mask's links before the mask.** A parent chain that ends at 0 or a parent cycle is fatal to a rebuild on the draw thread, so publish the mask first. Grow map and cache before linking.
- **Re-read the row after insertion with the gates written for the tree before it.** The per-frame path must accept the inserted tree, and `read_row` detects the marker for that.
- **Treat stage 20 or 21 as terminal.** The HUD is simply absent before a quest. Retry; only factory, allocation and publish failures are terminal.
- **Feed the box from the bar's authored 88 px.** That cut the blade near the tip at 190 ("worse than r33"). `bar_w=88` reads fine, but the rect's 128 is what the tester's cuts line up with.
- **Trust `eff=` at the mask's enter.** The bar's effective rect is not written yet at that point and reads `0 0 0 0`.
- **Quote a window that is not in a log.** `tex=0 156 87 440` was invented, and an axis theory was built on it. Decode the `+0x10c` floats.
- **Trust `work/modern-sharpness-gauge/scripts/xref.py` or `find_word.py`.** Both are broken and their "no `bl`
  site" results are void. Use `research xref`, which still finds no direct branch to the Priority accessors,
  the mask factory, the root destructor or the mask's enter.
- **Subtract the Ghidra base from vtable words.** `main+0x85628`, `0x86420`, `0x866a4` and `0x8677c` are that
  mistake. The texture's slots are 0x38 `0xa4e628`, 0x74 `0xa4f420`, 0x00 `0xa4f6a4`, 0x04 `0xa4f77c`.
- **Call the texture's slot 0x00 as a constructor.** It is not one, and it froze the game in the quest.
- **Resolve `Texture` before the copy has the bar's `+0x64`.** The setter scans `*(obj+0x64) + 0xe8`, the constructor zeroes that word, and the copy is refused at stage 52.
- **Treat `dim_flags=0x20a0001` as proof of drawing.** It is the constructor's default (main+0xa4d0d8).
- **Write atlas floats as integers.** That sampled `v` near 1.13e9. The words at `+0x10c` are floats: `280.0` logs as `1133248512`.
- **Write the mask's `+0xb0..+0xbc`, or remember the box between frames.** The rectangle is derived, and the game rebuilds it every frame. Write the inputs on every enter.
- **Read a mask's width from `bar+0xa8`, or leave its anchor nibble as allocated.** The first reads zero at enter; the second collapses the axis.
- **Write `+0xe0` on a mask.** The object is 0xd0 bytes, so that writes past its end.
- **Rely on a node's rect to clip it.** Zero-width `+0xe0/+0xf0` landed and the copy painted over the gauge anyway. Only the render state's scissor clips.
- **Expect a scissor to scale.** It clips at 1:1. r58 and r61 rested on a squeeze premise that is refuted; r61 is withdrawn, do not ship it.
- **Take a mask's corner from its parent's rect.** The builder main+0xa4b390 reads only the node's own transform, and the authored width spans forward from that corner.
- **Move the copy with raw position, size or window writes after the build.** They read back and never repaint. Use the setter for the window and a mask for geometry. The only build writing the quad per frame also faulted in the game's own update (`strb r1,[r4,#0xcb00]` at main+0x1014604, `r4` = 0, called from main+0xf28238). The mechanism is unread, so do not retry that without reading it.
- **Draw a fixed band.** The tester's requirement is the live tier frame: "r59 and r60 both failed to shown worn blade for orange or red color sharpness". The task wording "the shade always draws the unworn blade" had it backwards.
- **Link anything ahead of the mask on the controller's chain.** The row's chain went 8 ms after insertion and the row never matched again. Paint order is walk order, and the sibling after the mask is the only proven position.
- **Clear bit 17 on the clip mask, or call bit 19 a layout bit.** Without 17 the clip and copy keep their constructors' matrices. Bit 19 is main+0xa4915c's child re-sort on `+0x60`, so Priority is not build-time only.
- **Use the row's `row+4` pair for sizing.** The sizer uses `row+8`: 3 objects and 16 sequences here. The cache's objects field is the node count; the map is one longer.
- **Confuse the node's flag bits.** `0x00400000` is byte `+0x56` bit 6, which the texture's enter tests. The walker's exit clears bit 18 (`0xfffbffff`). Byte `+0x57` bit 1 is drawable bit 25. `+0x33` as a word index is `+0xcc`, the `Mask` property, not a kind word.
- **Use a timer or counter as liveness.** A freed mask's enter stops being called, so check from the walker.
- **Derive pixels from the bar's scale.** No mapping fits the measured ink, which is why the cut is a measured fraction of the rect.
- **Prove colour order with magenta.** It is symmetric. The bar's yellow `0xff5efaff` shows the word is `0xAABBGGRR`.
- **Read `mInputFlags` as a riding state.** That word is the tier.
- **Read the grey band under the gauge as this mod's.** It is the Dual Blades' demon gauge, a row this mod does not touch. Name each element in a photo.
- **Blame this mod for Eden's `maxwell_3d.cpp:392` asserts or a PANIC right after `ForceStop`.** Both also appear with no insertion, or as the emulator stops.

## Open items

- The draw walker's thread. `insert::run` changes the row from `forge_onUpdate`, after the game's update
  entry main+0x3da8fc (`sFestaMain::move`); the mask's writes happen on whatever thread reaches the walker, and
  the two paths share captured addresses through atomics with no lock. Unknown: a fix needs the thread first.
- `grow_map` leaks the old 16-byte map block. `FUN_01401bc0` allocates a row's map and `setAnimationId` frees it
  through the same allocator (GOT slot `0x18422b4`, the DTI registered as `cGUIInstance` at `0xa3c03c`; allocation
  `0xa38c14`, frees `0xa38880` and `0xa382c8`). Native readers such as `0x5456e4` and `0xa4915c` index the
  pointer straight away, so a second thread could read a released block. Fix it only in a runtime version that gets a run with an area change.
- Two comments in `insert.rs` are wrong, left until the next runtime version. The header says the insertion
  runs inside a native callback on the GUI thread, but `gui::tick` runs from `forge_onUpdate`. The `CLIP_MARKER`
  comment says only a scissor mask's enter writes the box through `0xa37c1c`; a texture's enter does too under `Mask` 3 or 6.
- An orange-tier crop (the accepted crop is red), a sharpening across tiers in one crop, and the dark part's
  missing halo, which is a design question.
- Unconfirmed: Ryujinx, native hardware, and a bar rect other than 110..238 (the scaled offsets have only been seen where they reproduce the measured numbers).
- Never exercised: the root destructor hook has fired in no saved log. ARM fixtures for insertion and allocation failure are not written.
