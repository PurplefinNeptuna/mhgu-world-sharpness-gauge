# World Sharpness Gauge: insertion and cut

Worked example from `mods/world-sharpness-gauge/`. Game facts it rests on are in `mhgu-understanding`.

## Draw block prefix

The row's draw block (`root+0xa4`) starts with `sequences * 12` bytes of animation tracks, 192 here. Row setup loads the block at
main+0xa38cfc and main+0xa484c4 stores each object's slice of it at `+0x64`, writing the object's address at `+8` of each 12-byte
track. The row update main+0xa4915c walks the first `row+4` map entries (controller, bar, glow, edge) and, for an object with bit 23,
evaluates its tracks through `+0x64` at main+0xb349c0. The copy is given the bar's `+0x64`, so it points there too. A replaced block
therefore keeps the prefix bytes and every `+0x64` inside `old..=old + prefix` moves to the same offset (`relocate_prefix` in core).

General GUI rules (masks, cache and map growth, teardown) are in the forge-modding skill's
`references/gui-insertion.md`; row, field and table facts are in the mhgu-understanding skill's
`references/sharpness.md`.

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
- Discovery cadence: one attempt every 60 walker calls, so a few fractions of a second per HUD. Nothing is queued; formatting and logging happen only in `forge_onUpdate`.
- Fill math, in `core`: require a maximum `> 0`, find the band by binary search on the supplied
  points-to-tier function, compute `(current - floor) * 10_000 / (ceiling - floor)` in `i64`, clamp
  to `0..=10000`, and keep every probe inside `0..=maximum`. A zero, negative, missing or
  above-maximum state yields no ratio; above maximum draws full, depletion floors at empty. Nothing
  is divided by an unverified band width, and no native call happens before `src/sampling.rs`'s gates.
- Native calls run only in the mask's enter handler, from `sampling::fill`: about 18 per frame
  (two searches over at most 450 points), each behind `memory::executable`. Nothing is cached, so a
  weapon change needs no invalidation; add a band cache keyed by `(points, maximum, tier)` only if a
  run shows a cost.

## Accepted shape

- **Scope, set by the tester.** The bar's length is the fill inside the tier band its points sit in.
  The tester's words: "the bar is per tier, so for example, weapon with purple tier max, the bar will
  show the fill as between max sharpness, and the sharpness point at white, then it became full again
  until it reach sharpness point for blue, etc.", "coloring is already managed by game, so you only
  need to control the bar", "dont forget any buff that might increase sharpness and might change how
  the calculation at max tier sharpness", and "thin band sharpness tier are okay, the need is
  approximation, so jumpy is okay, no need for fancy smooth animations or lerp". So the computed fill
  is published directly: nothing is smoothed, interpolated or animated. The mod picks no colour,
  touches no animation sequence, and leaves the native HUD alone in every unsupported state.
- **Tree.** The controller's first child is the mask over bar, glow and edge; the clip mask is the mask's
  sibling, with the copy as its child. The walk emits mask enter, bar/glow/edge enter+exit, mask exit, clip
  enter, copy enter/exit, clip exit, terminator. Neither mask clips (see *Geometric cut*); the walk order
  is what puts the cut mask's enter, where the frame's cut is decided, before every sprite's draw. Teardown follows the same links: the mask destroys bar, glow and edge, and its sibling chain destroys
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
- **Geometric cut.** The masks are built and written as before but carry `+0xa4 = 2`: the box is rebuilt and
  no scissor is applied (main+0xa4bbe0) or restored (main+0xa4bce4). The cut mask's enter sets the frame's
  span in the space of the bar's own `+0x10` matrix (`node_rect`, `drawn_rect` over the node's words): the
  bar, glow and edge keep canvas x up to `rect left + cut_box` width, the copy keeps `dim_box`'s left..right.
  `wsg_texture_draw` cuts each of the four before the original: `crop_span` takes the quad as main+0xa4feb4 builds it (the float
  authored width at `+0xb4`, anchor `+0xa0 & 3`; the layout width at `+0xd8` is the rect, not the quad), scales
  `+0x10` (m[0]) by the kept fraction, moves `+0x40` (m[12]) so the kept part stays in place, and gives the window `[u1, v1, u2,
  v2]` the same fraction through the setter; after the original it puts m[0] and m[12] back. The draw
  uploads `node+0x10` at main+0xa4fd10/0xa4fd38 and rebuilds the UVs at main+0xa4fe44, so both are drawn
  this frame, and anything that moves the drawn quad later (UIScale's block edit, gu-native's renderer)
  moves the cut with it. A cut window stays on the sprite between draws; a live window other than the one
  set is the game's new one, so the setter runs only when the game's window or the cut changes. The copy
  takes the bar's game window. A sprite `crop_span` cannot model is drawn whole with the game's window, and
  the copy then not at all. The log line `geometric cut` counts the cut draws per sprite once per row.
- **Shade.** Each frame the clip mask's translation (`+0x40/+0x44`) is the cut's right edge and the bar's
  rect top; its authored width is the blade past the cut and its height the bar's rect height. r62 logged
  `shade_box=128 38 200 166` at `ratio_bp=0`. These writes are kept from r68 so the copy is placed as it
  was; the copy's cut is the geometric one. The dark part has no glow by design.
- **Following a rescaled bar.** UIScale edits a sprite in the uploaded matrix block at `*(ctx+4)+0x18bc`, after main+0xa4eb9c or main+0xa3e3b0, so the node's own `+0x10` stays unscaled. Its rewrite of `+0xe0` (main+0xa4f420's output) refreshes only on a rebuild and takes a different anchor branch from the sprite (`+0x22` on a hit, UIScale `0x5200`). So `wsg_texture_draw` reads the block after the original returns, which holds in either hook order. A block address unchanged by the draw means nothing was uploaded. Words 0, 1, 4, 5, 12 and 13 equal to the node's mean a plain draw, and the game's rect stays the source. Otherwise `drawn_rect` gives the box, one frame old at the mask's enter. Each frame the mask's enter writes the box's corner over the mask's translation (`+0x40/+0x44`), the original builds the box, and the hook puts the walk's value back, because a child's setMatrix reads its parent's `+0x10` (main+0xa496b0). Since r69 this box only places the masks and feeds the `bar drawn` line: the drawn words are the cut quad's, so `capture_drawn` takes the game's saved words through the drawn x scale and shift to report the whole bar. The cut itself follows a rescaled bar because UIScale edits the already-cut quad.
- **The copy under UIScale.** UIScale picks the element from KatLib's stack of instances being drawn (`0x4138`), not from the node, and keys the anchor box on that instance (`0x13b8`), so the copy and the bar get one transform.
- **Copy construction.** DTI allocator slot 0x20 with `(bytes, align, dti id)`, a zeroed 304-byte block,
  constructor main+0xa4f60c, the mod's marker at `+0x50`, the bar's `+0x64/+0x74/+0x78`, the twelve style setters in Key Quest
  Markers' order, `SamplerState` from the bar (authored 2, stored in the low nibble of `+0x108` by
  main+0xa4fa5c), the `Texture` id last, then the window through the setter. Stages 50..52 are additive: no
  copy, and the gauge looks like the plain cut.
- **Transform.** Bit 17 of `+0x54` is transform dirty. The walk main+0xa4964c, started by main+0xa39c40 when
  root `+0x8c` bit 1 is set, calls setMatrix (slot 0x44, main+0xa4e604 for a texture) only on a node with
  that bit, and pushes the bits to children only from a dirty node. Mask defaults come from factory
  main+0xa4b6cc, which is reached through the class table at main+0x178ff04.
- **Lifecycle.** A gauge row lasts one quest. Inference: 21 of 24 row ends across 19 saved logs land within 40 s of the game's save commit (the other 3 sit near the end of their logs), the shortest gap from a row's end to the next insertion is 124 s, rows live 240..1827 s apart from one of 59 s that also ends beside a save commit, and testers report area changes and faints with no re-insertion. No log line marks an area change; the count is in `work/world-sharpness-gauge/ROW_LIFETIME_AND_FIXED_TIER_RESEARCH.md`. The walker hook checks the marker and links read-only; on failure it runs `hooks::teardown_gauge_row`, so the next row is read and inserted like the first. The mask's enter disables a mask whose links are gone. Forge has no plugin unload, so nothing is undone at removal.
