# World Sharpness Gauge: development

**Current state.** r70 READY_TO_DISTRIBUTE (2026-11-10), unpublished, release at `build/world-sharpness-gauge/releases/r70/`. It is the last gameplay-accepted build: the user reported a full gauge with the dark part in place on MHGURecomp at 1920x1080. Eden and Ryubing were not run with r70; r68 at `build/world-sharpness-gauge/candidates/r68/` is the last build accepted there ("good on both"). r67 at `build/world-sharpness-gauge/candidates/r67/` is the accepted run on MHGURecomp 0.14.0 beside KatLib and UIScale. The accepted picture is r64's (`build/world-sharpness-gauge/candidates/r64/`, Eden, "all works perfectly"), and the accepted crop of the worn blade at the red tier is r62's (`build/world-sharpness-gauge/candidates/r62/`). `DISTRIBUTION.json` holds the status and hashes, `EVIDENCE.json` every version's results, `TESTING.md` the tester steps and log lines.

How the current build draws:

- Each crop is sized from the quad the draw builds (the float authored width at `+0xb4`), not the 128 px layout width at `+0xd8`.
- The cut and the shade are the sprites' own geometry, not scissors (see Geometric cut in [Accepted shape](../../.agents/skills/mhgu-forge-modding/references/worked/sharpness-gauge.md#accepted-shape)), because gu-native, an Android and Windows port, moves the HUD's quads in its renderer and leaves the game's scissor where it was. The masks' scissor is off.
- `grow_cache` copies the draw block's animation prefix and moves every object's track pointer (`+0x64`) into the new block before freeing the old one (see [Draw block prefix](../../.agents/skills/mhgu-forge-modding/references/worked/sharpness-gauge.md#draw-block-prefix)).
- A required code window may carry another plugin's Forge entry hook over its first eight bytes, because all four hooks chain to the original (`window_matches` in core).
- The cut and the shade follow where the bar is really drawn: a post-hook on the texture draw reads the bar's uploaded matrix, and a plain draw keeps the game's rect as the source.
- The bar's length is measured inside the tier band its points sit in: `(current - floor) * 10_000 / (ceiling - floor)`, where `floor` is the top of the tier below and `ceiling` the top of this tier clipped to `mMaxSharpness`, found by binary search on `calculateSharpnessType` (main+0x29cdfc), which is called, never hooked (see [Fill, per tier](../../.agents/skills/mhgu-forge-modding/references/worked/sharpness-fill.md#fill-per-tier) below). No settings, no panel.
- The shade's atlas window is the bar's own live four floats, published through the `mTextureRect` setter every frame. The shade is the clip mask's sibling, one clip mask with the copy as its child, `#202326` = `0xFF262320`, the clip mask carrying bit 17, the copy carrying the bar's sampler.
- 6 tree nodes, the 7-word object array, `(6 << 4) | 8` = 104 bytes = 13 records, and the walk emits exactly 13.

The name is World because the per-tier bar works like Monster Hunter: World's, while the later games' long
gauge spans all sharpness. Builds through r64, the log lines quoted in `EVIDENCE.json` and `DISTRIBUTION.json`,
and the notes in `work/` use an earlier name and the log prefix `MSG`. An r64 folder must not be active
beside a later build's, because both plugins would load and hook the same functions.

The window is published through the setter because the drawn quad's UVs are a cache: the texture's draw
slot main+0xa4fc9c calls main+0xa4feb4, which reads the UVs from `+0x10c..+0x118` and the four corner
colours from `+0xa4` only while `(global+0x98 & (node+0xc0 >> 24) & 7)` is nonzero and clears those bits
afterwards, and the setter main+0xa4fb58 - early return while the four floats match, store, slot 0x28 when
`+0x108 & 0x30`, tail-call of vtable slot 0x50 (main+0xa4fc68) - is the only path to that mark. A raw
write to the copy's `+0x10c` marks nothing and never repaints. `work/world-sharpness-gauge/PHASE5_RESEARCH.md`
carries these facts with their bytes.

## How it works

Design, hooks and native detail live in the skills: [World Sharpness Gauge: insertion and cut](../../.agents/skills/mhgu-forge-modding/references/worked/sharpness-gauge.md), [World Sharpness Gauge: fill per tier](../../.agents/skills/mhgu-forge-modding/references/worked/sharpness-fill.md).

## Constraints

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/world-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.

## Commands

`./build.sh fetch|doctor|build|test|verify|package|export --output DEST`. There is no
`prepare`: this mod ships no resources, assets or INI, and has no settings,
so no Plugin UI panel: the build exports `forge_onLoad`, `forge_onInit`,
`forge_onUpdate`, `init` and `fini`, and `verify.py` requires exactly that set.
Inputs come from `build.local.json` (`main`, `sdk`, `forge` under `inputs`,
copied from `work/shared/game-exefs/`); `build.local.example.json` documents
the keys. Outputs go to `build/world-sharpness-gauge/candidates/VERSION/` and
scratch to `work/world-sharpness-gauge/candidates/VERSION/`, where VERSION is
`scripts/paths.py`'s current version. `package` runs the builder for a
`DEVELOPMENT` spec and refuses a `READY_TO_DISTRIBUTE` one.
`./build.sh test` runs the host fixtures for `x86_64-unknown-linux-gnu` and then
`verify`, so it must run after `build` and before `package`.

The native facts the fill rests on are re-checked against the original bytes by
`work/world-sharpness-gauge/scripts/verify_tier_gauge.py`: the 0x29cdfc window and that it is unique
in the image, its ABI and failure returns, the gate's two unchecked dereferences, the tier rule's
seven strict compares, the init's maximum formula and the two field offsets, and all 164 kireaji rows
for monotonicity, with the tester's log samples and the brief's example weapon carried as fixtures.

## Files

- `core/src/lib.rs`: `#![no_std]` decisions only: the local-player chain, the
  gauge read, the band search and the fill in basis points, the gauge-HUD
  identity, the cut box from the accepted rect offsets, the node count and cache
  sizing the insertion gates on, and the walker port the fixtures pin. No Forge,
  no FFI, no `unsafe`: the band arrives through a `&dyn Fn(i32) -> Option<u32>`
  the plugin fills with the native call.
- `core/tests/behavior.rs`: 27 host fixtures over a fake memory map and plain tier rules.
- `src/hooks.rs`: the four Forge hooks (walker, instance draw, mask enter, texture draw), their signature gate and rollback.
- `src/sampling.rs`: discovery in the walker's hot path: the slow attempt
  cadence, the captured controller and player, the gauge-present gate, the capture's
  teardown, and the per-frame fill with the three gates in front of its native calls.
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

The native functions this mod reads, hooks or calls (draw walker, controller setup, instance draw, scissor routine, texture enter and draw, transform walk and row update, the sharpness functions and gates, setters, linkers, mask enter and exit, destructors, the mask factory, `setAnimationId`) are listed with their proof in `.agents/skills/mhgu-understanding/references/gui-resources.md` and `sharpness.md`.

Runtime state read by the hook:

| Location | Meaning |
| --- | --- |
| `main+0x188ca48` | session singleton; `+0x30` is the local hunter index, valid 0..3 |
| `main+0x1885b58` | `sPlayer` singleton; `+0x1c + 4*index` is the local player pointer |
| player `+10100` | `uPlayerQuest::mSharpness`, signed points |
| player `+10104` | `uPlayerQuest::mMaxSharpness`, signed points: `min(stat 14 * 25 + 150, 450)`, written once by init and by nothing else, so it never changes inside a quest. Stat 14 (`s16` at `mpPlParam + 0x338`) carries the weapon's sharpness level plus the skill bonus, and `0xe1eec` caps it at 12 |
| player `+5104` | `uPlayerBase::mSharpnessType`, unsigned tier index: 0 red, 1 orange, 2 yellow, 3 green, 4 blue, 5 white, 6 purple (mapping from the tester, 2026-10-22). A tier is a per-weapon segment of points. The fill does not read this field: 67 sites build the offset and only about 10 were opened, so its writers are not fully known |
| player `+0x13c4` | the param cell `calculateSharpnessType` loads; the byte at that pointer `+ 0x4d4` is the sharpness state its gate reads. Both are read by the fill's third gate before the first native call |
| controller `+0x68/+0x80/+0x8c/+0x9c/+0xa0` | tree root, draw cache, rebuild flags, animation row, map alias |
| row `+0/+4/+0xc/+0x14` | animation id; counts packed in one word (objects low u16, sequences high u16); first object row; the word at `+0x14` is a pointer into the sequence block, not a row index |
| node `+0x54` | flag word: `.agents/skills/mhgu-understanding/references/gui-resources.md` has the bits |
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

## Sharpness chain

`adjustSharpness` events, skill decay rules, the tier lookup through weapon data, `WpDatEntry` and `mCmdTblId` are in `.agents/skills/mhgu-understanding/references/sharpness.md`.

## Do not

- **Cut the gauge with a scissor.** A port that moves the HUD's quads in its renderer (gu-native's `exec.c` rewrites each quad's matrix and moves no status scissor) leaves the cut in the game's pixels while the bar moves.
- **Free or zero the draw block's prefix while objects point into it.** The bar, glow and edge animate through `+0x64` every frame; r67 did, and Ryubing crashed at main+0xb349c0 from main+0xa49238.
- **Call native code on a player that has not passed the fill's gates.** `calculateSharpnessType`
  dereferences `player + 0x13c4` (main+0x29ce34) and then `+ 0x4d4` (main+0x27c848, main+0x27c84c)
  with no check, so a stale capture can fault the game. `sampling::fill` re-derives the player from
  the session and requires it to equal the capture, requires a positive maximum, and requires both
  pointers to read, in that order, before its first call.
- **Take the maximum from stat 14, a level row or the kireaji table.** `mMaxSharpness` is what the
  game clamps points against. `0xe1eec` is also reached from the player update path `0x27b9c8`, and
  rerunning it does not move `mMaxSharpness`.
- **Size the cut from `+0xe0` alone under a rescaling plugin.** It goes stale between rebuilds and is not guaranteed to match the sprite. The uploaded block is what was drawn.
- **Leave a moved corner on the cut mask past its enter.** The bar is its child, so the next transform update of the bar alone would read the moved parent matrix, and a rescaling plugin would scale that again.
- **Read `mSharpnessType` for the fill.** The stored tier is not the band, and 67 sites build that
  offset while only about 10 were opened, so its writers are not fully known.
- **Smooth, interpolate or animate the fill.** "jumpy is okay, no need for fancy smooth animations
  or lerp" (tester). The computed fill is published as it is.
- **Cache the band between frames.** It depends on the points, the maximum and the game's gates,
  so a cache needs its own invalidation; the ~18 native calls per frame have no measured cost.
- **Leave the glow and edge outside the mask.** A mask whose tree never closes has no exit record, so a scissor it applies never ends; `a_mask_left_open_never_stops_clipping` pins that shape. `+0xa4` bit 1 only rebuilds, and bit 0 asks for the scissor: r69 keeps bit 0 clear on purpose.
- **Publish a mask's links before the mask.** A parent chain that ends at 0 or a parent cycle is fatal to a rebuild on the draw thread, so publish the mask first. Grow map and cache before linking.
- **Re-read the row after insertion with the gates written for the tree before it.** The per-frame path must accept the inserted tree, and `read_row` detects the marker for that.
- **Treat a loading screen as a row's end.** A gauge row lasts a quest (see Lifecycle), and older notes calling a row's end "the area transition" were labels on events beside a save commit.
- **Rely on the `cGUIObjRoot` destructor to report a row's end.** A hook on main+0xa486fc fired in no saved log while the mask's link checks caught every row's end.
- **Treat stage 20 or 21 as terminal.** The HUD is simply absent before a quest. Retry; only factory, allocation and publish failures are terminal.
- **Feed the box from the bar's authored 88 px.** That cut the blade near the tip at 190 ("worse than r33"). `bar_w=88` reads fine, but the rect's 128 is what the tester's cuts line up with.
- **Trust `eff=` at the mask's enter.** The bar's effective rect is not written yet at that point and reads `0 0 0 0`.
- **Quote a window that is not in a log.** `tex=0 156 87 440` was invented, and an axis theory was built on it. Decode the `+0x10c` floats.
- **Trust `work/world-sharpness-gauge/scripts/xref.py` or `find_word.py`.** Both are broken and their "no `bl`
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
- **Rely on a node's rect to clip it.** Zero-width `+0xe0/+0xf0` landed and the copy painted over the gauge anyway. A scissor clips, and so does the sprite's own `+0x10` matrix and window at its draw (*Geometric cut*).
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

- **A live drag in UIScale's editor.** The quest beside KatLib and UIScale worked on MHGURecomp, and the log has many `[UIScale] settings saved` lines, but the report does not say the cut followed a drag. On Eden both plugins fail to load with `0x00000616` beside BetterBox, because all three NROs have an all-zero build ID.
- **Steps not seen on screen.** Sharpness +1/+2 and Heavy Polish over the banded bar have host fixtures and static reading, and no run. `TESTING.md` has the steps if anyone plays them.
- **The bar and the colour on the same frame.** The colour driver main+0x62bb0c (slot 0x3c of the widget vtable at main+0x1776300) reads the tier, stores its sequence at `+0x50` and applies it through main+0x546c74 and main+0xa3896c in that one call. When that call runs against `adjustSharpness` and against the draw is unread; it depends on the unit move order (`sUnit::registerUnit` main+0xc067ec takes the line). A tier drop could show a full bar in the old colour for a frame or more. The player reports nothing wrong, and only a slow capture can show it. Read the order only if it is ever seen.
- **The draw walker's thread.** One thread by static reading, unconfirmed at runtime; the fill makes about 18 checked native calls per frame on it. `forge_onUpdate` runs after `sFestaMain::move` (main+0x3da8fc) returns, and that function reaches the walker inline: render slot 0x1c main+0xbc2904, unit manager slot 0x34 main+0x3ea0b0, uGUI slot 0x30 main+0x919760, main+0xa34070, instance slot 0x3c main+0xa39300. So `insert::run` follows the walker and the mask's enter rather than overlapping them. Untraced: 13 other unit-draw callers and 10 other slot 0x3c sites, and the per-frame job main+0x3daf88, which can outlive `forge_onUpdate` while its sUIObject pass destroys `cUIO*` objects. Only a test build comparing the current thread in the walker and in `forge_onUpdate` can confirm it. Offsets are in `work/world-sharpness-gauge/THREAD_RESEARCH.md`.
- **The `fill refused` log line.** It has never been seen in a run. A session chain that drops before the mask's links do would print it at a normal quest end, so it means a failure only when the stock bar shows during a hunt.
- **`grow_map` leaks the old 16-byte map block**, once per quest, which never matters in a session. `FUN_01401bc0` allocates a row's map and `setAnimationId` frees it through the same allocator (GOT slot `0x18422b4`, the DTI registered as `cGUIInstance` at `0xa3c03c`; allocation `0xa38c14`, frees `0xa38880` and `0xa382c8`). Native readers such as `0x5456e4` and `0xa4915c` index the pointer straight away, and `0x5456e4`'s thread is untraced, so any fix needs a run.
- **The dark part's missing halo** is a design question left open.
- **Unconfirmed:** Ryujinx, native hardware, and a bar rect other than 110..238 (the scaled offsets have only been seen where they reproduce the measured numbers). ARM fixtures for insertion and allocation failure are not written.
