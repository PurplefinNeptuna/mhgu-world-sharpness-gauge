Current state: run **r53**, the preserved fallback at `build/modern-sharpness-gauge/candidates/r53/`
(bytes unchanged, sha256 of its zip `3130d2c0e8a12fdc602a5f23b4b81adb65895aed14e1f7edfd01e866e5b982ab`).
It is r52's proven tree shape - the mask as the controller's first child, the shadow as the mask's
sibling, drawn last, outside the scissor - with the shadow carved to the part of the blade past the cut
and coloured `#202326` (`0xFF262320`).

r54 (the shadow behind the gauge) is recorded as a regression and is not the candidate. Its log
(`work/modern-sharpness-gauge/runs/r54-eden-2026-10-31.txt`, sha256
`26ae7bdb1e151959a7ba00534438460b07263dd12128baef8ee0e03827e8b87f`) has one insertion at t=112.097788
(`entries=6 cache=0x48868c10 records=11 dim=0x48872f70 dim_tex=0x517abab0`), and 8 ms later the mask's
links are gone: `mask.parent=0x488466f0 mask.child=0x48857330 bar.parent=0x0 bar.sibling=0x0
root.child=0x0`. That line then repeats 17104 times to the end of the log, each time paired with
`no sharpness gauge controller seen in N walker calls (HUD absent?)` (17105 lines, N from 48212 to
576683), and the log has **zero** fill lines and zero state lines: the enter hook never reached the
live-fill path, so every entry took the links-lost path, which disables the mask. The bar is never
clipped, which is the tester's photo: the full uncut blade, no shadow.

The insertion point is the difference. Putting the shadow ahead of the mask on the controller's chain
is the only link position that draws it before the bar, and the row's own chain teardown followed 8 ms
after that insertion. The shape that ran clean and rebuilt across areas is r52's: mask first, shadow as
the mask's sibling. r54 stays in the history with the layering research it produced - paint order is
walk order (main+0xa494a0 emits and consumes in visit order, no sort, no z value), and the GUI file's
`Priority` (bar 0, glow 1, edge 2) ordered the chain at build time and is not read by the walker - and
with this: the layer is reachable, the position is not safe.

r54 DEVELOPMENT (2026-10-31), superseded by the r53 fallback after its run:
Current state: r54 DEVELOPMENT candidate, the plan's Phase 5, the shadow behind the gauge. The tester
asked why the shadow is not simply the full blade image behind the gauge, and the layering research
says that is what the game does with its own row: main+0xa494a0 appends `(0,node)` on entry and
`(1,node)` on exit in the order it visits nodes and then consumes that list in order - vtable 0x38 and
0x3c on kind 0, 0x40 on kind 1, return on kind 2 - with no sort and no z value anywhere in it. Paint
order is walk order. The row's `Priority` values in the resource (hud0: `Txt_sharp` 0, `Txt_sharpefx`
1, `Txt_sharpEdge` 2, and the runs walk bar → glow → edge) are what ordered that chain when the row
was built; the walker never reads the property, and `xref.py` finds no `bl` site for its accessor
(main+0xa47a04 / main+0xa479fc, the field pair `+0x5c/+0x60`, propagated to the parent), so it is
build-time ordering. A node inserted at runtime gets its layer from where it is linked.

So r54 links the shadow first: `root.child = shadow`, `shadow.sibling = mask`, mask over the bar,
glow and edge as before. The walker emits the shadow's pair before the mask's enter record, where the
scissor starts, so the shadow draws under the gauge and outside the clip. Its quad is the bar's own
pair and the bar's authored 88 x 24, written once at the build; nothing is written per frame for
placement, and the cut does the work - the bar's ink covers its kept part and the sprite's own shape
shows past the cut. The colour is the tester's `#202326` (`0xFF262320`, the alpha, blue, green, red
order the r52 run proved). The only per-frame write left is the atlas band: the four words at
`+0x10c` follow the bar's live tier band, so the dim blade changes colour with the tier. The mask's
enter is visited after the shadow's records, so a tier change shows on the shadow one frame late;
writing it in the shadow's own enter (this class's vtable slot 0x38, main+0x85628) needs a hook on a
shared class function, gated on the marker word, and nothing measured asks for it yet. Storage is
unchanged: 5 tree nodes, the 6-word array, 88 bytes = 11 records, and the fixture now pins the shadow
pair as the walk's first two records with the mask's enter third and the mask's exit closing the
scope. 21 host fixtures pass; verify PASSED with the same 33 signature windows. Gameplay NOT RUN.
**r53, the shadow carved to the hidden part in the gauge's band, is kept as the fallback candidate at
`build/modern-sharpness-gauge/candidates/r53/` - its bytes are unchanged. If the behind version shows
a dark rim on the kept side, r53 is the version to run.**

r53 DEVELOPMENT (2026-10-31), superseded by r54:
Current state: r53 DEVELOPMENT candidate, the plan's Phase 5. r52 answered its question: the copy is
on screen. The tester's photo shows a full-length magenta blade under the sharpness gauge, and it
stays in place as sharpness drops and recovers and across areas. Two things that run settled: the
node type, recipe and link draw, and the `MT color` word's byte order - the constant written as
`0xFFFF00FF` came out magenta, so the bytes are alpha, blue, green, red, and the shadow colour the
tester asked for, `#202326`, is `0xFF262320`.

r53 puts the shadow where the plan wants it: in the gauge's own band, covering the part of the blade
past the cut. It is written every frame from the accepted cut geometry, in the units the node's own
`+0x80/+0xb4` pair uses. The bar's authored 88.0 covers its rect's whole width (the accepted runs log
`bar=110 38 238 166` with `bar_w=88`), so a length of `px` in the rect is `px * 88 / rect width` in
authored units, and its position pair maps 1:1 onto the rect's corner (the r8 run read 284.0 / 224.0
at rect 110 / 38). Core's `dim_quad` does that conversion and the new fixture pins it: at 0% on the
measured bar the shadow starts 18 units along, is 49.5 authored units wide and samples from column 12
of the band the bar is drawing; at 50% it starts at 54 and is 24.75 wide from column 37; at 100% the
hidden width is 1 px, the sprite's empty tail past the tip. With no gauge to cut by it goes to width
0, the accepted appearance. The mask's enter reads the copy's and the bar's own state back before
each write, so the run measures what the layout made of the quad. 22 host fixtures pass; verify
PASSED with the same 33 signature windows. Gameplay NOT RUN.

r52 DEVELOPMENT (2026-10-31), superseded by r53 after its run:
Current state: r52 DEVELOPMENT candidate, the plan's Phase 5, a forced-visibility candidate. The copy
has never been seen where it was put, so r52 stops tuning geometry. The copy gets Key Quest Markers'
complete badge recipe: the allocator slot and constructor it uses, the bar's `+0x50`/`+0x64`/`+0x74`/
`+0x78`, the twelve style setters in its order with its values (`BlendState=1`, `ControlPoint=0`,
`FixFrame=0`, `Mask=0`, `Priority=2`, `ReferencePosition=1`, `ReferenceRotation=1`,
`ReferenceScaleX/Y/Z=3`, `Reverse=0`, `SamplerState=0`), then the `Texture` id last, then its atlas
rectangle through `mTextureRect` (main+0xa4fb58) with a pointer to the bar's own live four floats.
Then one set of forced values, written once at the build and never touched again: the bar's position
pair with 32.0 added to y, the bar's own authored pair as its size (88.0 x 24.0), and opaque magenta
`0xFFFF00FF` at all four corners, since the bar's own `0xff5efaff` decodes as yellow in AABBGGRR and
magenta is a colour this gauge has none of. It stays the mask's sibling, so it draws after the mask's
subtree and outside the scissor. Nothing is written to it per frame: the per-frame quad and the
`fill copy` line are gone. In their place, the mask's enter reads the copy's game-computed state
back, before any write, and the same fields on the bar: the rect at `+0xe0`, the effective rect at
`+0xf0`, the 16-word matrix at `+0x10..+0x4c`, the position pair at `+0x80/+0x84` and the authored
pair at `+0xb4/+0xb8`. The lines print on change, rate-limited, and floats print as floats; the cut
line's `tex=` prints floats now too, which is what the words at `+0x10c` are.
Prediction for the run: a magenta full-length blade, 32 units under the sharpness gauge, no pulse.
The read-back values that confirm it: the copy's position pair at 284.0 / 256.0 on the measured bar
(the bar's own pair with the drop), its authored pair at 88.0 / 24.0, its rect 88 wide and 24 tall
where the earlier runs' rectangles came out 0 wide, its matrix's x row 1.0 like the bar's (so the
authored 88 lands as 88 px), and its effective rect equal to its rect, because nothing clips it
outside the mask. If the copy's rect reads 88x24 at that position and no magenta blade is on screen,
the node is laid out and the gap is the draw path, so the next step is static research on the walker
main+0xa494a0 for why a sibling of the scissor mask is not emitted - not another r53. 23 host
fixtures pass; verify PASSED with 33 signature windows, the twelve setter windows Key Quest Markers
verifies now checked here as well. Gameplay NOT RUN. The MT `color` word's byte order is unproven, so
the magenta constant's alpha byte position is an inference.

r51 DEVELOPMENT (2026-10-31), superseded by r52:
Current state: r51 DEVELOPMENT candidate, the plan's Phase 5. The axis was mine, not the game's:
the sprite's own record (PHASE0: `Txt_sharp`, Size 88x24, atlas `0,280,88,304`, with tier bands
`0,304,88,328` orange and `0,328,88,352` red) says the blade's **length** runs along `u`, the sprite's
88 columns, and `v` carries the tier colour bands that the row's animation slides. The bar's own live
window in the log is one whole band: `tex=0 1133248512 1118830592 1134034944` is `[0, 280, 88, 304]`
and `tex=0 1134034944 1118830592 1134821376` is `[0, 304, 88, 328]` (the log prints the float words
at `+0x10c` as integers). r48 put the copy's window on `v` between the strip's tip end and the bar's
current `v1`, which at mid sharpness is the band above the bar's own.
r51 puts it back on `u`: the copy takes the columns the cut hides, from the band the bar is drawing
(so the same sprite in the same tier colour), drawn from the cut's right edge to the tip's screen
position - the 100% cut box's width, 90 px on the measured bar - not to the bar rect's right edge.
The r50 run proved the placement: its quad line reads `fill copy rect=157 38 200 166 pos=47
authored=43` at about 41% sharpness and `rect=200 38 200 166 authored=1` at full sharpness, with no
fault and the accepted cut running normally, and the tester sees no copy in the sharpness gauge.
The copy's `dim_flags=0x20a0001` on the insertion line is the class constructor's own default
(main+0xa4d0d8 writes 0x020a0001 at +0x54), so it proves construction, not drawing.
The construction (allocator slot 0x20 with the DTI id,
zeroed 304-byte block, constructor main+0xa4f60c, the bar's owner/resource/`+0x64` cGUI, texture
resolved last, flat grey at the four corners, placement as the mask's sibling) and the per-frame quad
in the bar's space (position = the bar's position plus the kept width, authored width = the hidden
blade's width) are unchanged from r47/r50. 22 host fixtures pass; verify PASSED with the same 21
signature windows. Gameplay NOT RUN. The MT `color` word's byte order is unproven, so the grey's
alpha byte position is an inference.

## Superseded current-state paragraphs

r50 DEVELOPMENT (2026-10-31), superseded by r51:
Current state: r50 DEVELOPMENT candidate. The plan's Phase 5 copy has never been seen in the
sharpness gauge: the r47 and r48 runs both log it built with its texture resolved (`records=11`,
`dim=0x…`, `dim_tex=0x…`), with no fault and the accepted cut running normally, and the tester sees
nothing of it in the gauge. The `dim_flags=0x20a0001` on those lines is the class constructor's own
default - main+0xa4d0d8 writes 0x020a0001 at +0x54 - so it proves the constructor ran, and nothing
about drawing. In the r48 photo the grey band above the red cropped blade is the Dual Blade's demon
gauge, a row this mod does not touch; the noisy dark translucent block in the r47 photo is read as
this mod's copy, which is an inference and not a log fact. r49 fixes the placement: a `cGUIObjTexture` is laid out from its
position pair (+0x80/+0x84) and authored pair (+0xb4/+0xb8) when the walker visits it, after this
plugin's callback at the mask's enter, so r48's per-frame rectangle writes lose to that layout, and
r48's position pair held screen pixels (110, 38) written into parent-space offsets. r49 moves the quad
in the bar's own space: position = the bar's position plus the kept part's width, authored width =
the hidden blade's width, the bar's y and band height, layout dirty bit set. r50 prints that quad on
the log (`fill copy rect=x y x2 y2 pos=... authored=...` under each cut line), so the next run reports
the placement from the log instead of from a photograph. 22 host fixtures pass; verify PASSED with
the same 21 signature windows. Gameplay NOT RUN. The MT `color` word's byte order is unproven, so the
grey's alpha byte position is an inference.

r49 DEVELOPMENT (2026-10-31), superseded by r50:
Current state: r49 DEVELOPMENT candidate. The r48 run reported `dim=0x48043500 dim_tex=0x5194bab0
dim_flags=0x20a0001` with `records=11`, no fault and 62 normal cut lines. Nothing of the copy
appeared in the sharpness gauge. The grey blade-shaped band with the slanted tip in that run's photo
is the demon gauge's own graphics, as the tester said, and the flag word is the constructor's default
(main+0xa4d0d8 writes 0x020a0001 at +0x54): this paragraph's earlier reading of the photo as the copy
was wrong. The layout defect is one thing: a `cGUIObjTexture` is laid out from its position and authored size, and
that layout runs on the node's own visit in the draw walk, after this plugin's callback, so writing
the node's rectangle per frame gets overwritten and the copy comes out at the authored 88 px starting
from wherever its position points. r48 also placed that position with screen coordinates in fields
that read them as offsets in the parent's space, so the copy lands far from the blade, which is what
the gauge showing nothing of it means. r49 moves
the quad in the bar's own space: the copy's position is the bar's position plus the kept part's width,
its authored width is the hidden blade's width, and its y and band height stay the bar's. The sprite
rows and the accepted cut are unchanged from r48 (the bar's `u` pair, its `v` from the strip's
tip end - the smallest `v1` the row has drawn - down to the bar's current `v1`). Construction is r47's
(proven in play): the class DTI's allocator slot 0x20 with the DTI id, a zeroed 304-byte block, the
standalone constructor main+0xa4f60c, the bar's owner, resource, `+0x64` cGUI, authored pair and
drawable flags, the texture resolved last, flat grey at the four corners, placed as the mask's
sibling. 22 host fixtures pass; verify PASSED with the same 21 signature windows. Gameplay NOT RUN.
The MT `color` word's byte order is unproven, so the grey's alpha byte position is an inference.

r48 DEVELOPMENT (2026-10-31), superseded by r49 after its run:
Current state: r48 DEVELOPMENT candidate. The r47 log reports `dim=0x44246540 dim_tex=0x519bbab0
dim_flags=0x20a0001` with `records=11` and no fault: the copy exists and resolved its texture. The
flag word is the constructor's own default (main+0xa4d0d8 writes 0x020a0001 at +0x54), so the line
proves construction, not drawing. The noisy strip near the bar in that run's photo is read as the
copy's drawing - an inference, the only build whose copy could not sample its texture: r47 wrote the
bar's rect corner (110, 38) into the copy's local position at +0x80/+0x84, and wrote its atlas
window from words read as integers, so the bar's float 280.0 (the logged word 1133248512) went in as
1133248512.0 and the copy sampled around v = 1.13e9, outside the 88x24 sheet: a block of sheet
noise. r48's own run shows nothing of it. What the bar's window is, read from the log and decoded as
floats: `tex=0 1133248512 1118830592 1134034944` is [0, 280, 88, 304], and the same line at lower
sharpness reads `tex=0 1134034944 1118830592 1134821376` ([0, 304, 88, 328]) and, in the r48 run,
`tex=0 1134821376 1118830592 1135607808` ([0, 328, 88, 352]). The `u` pair is fixed at 0..88 and the
`v` pair steps by tier band, not by sharpness; the window `tex=0 156 87 440` quoted in the earlier
version of this paragraph is in no log. r48 takes the copy's rows from the same window the bar uses:
the sprite's `v` from the strip's tip end down to the bar's current `v1`, drawn
from the cut's right edge to the tip's screen position, with the bar's own `u` pair. The tip end is
read live as the smallest `v1` the bar has drawn, so a row whose player never reached full sharpness
draws nothing rather than guessing at the sprite. Construction is r47's, proven by that log: the
class DTI's allocator slot 0x20 with the DTI id, a zeroed 304-byte block, the standalone constructor
main+0xa4f60c, the bar's owner, resource, `+0x64` cGUI, position, authored pair and drawable flags,
the texture resolved last, flat grey at the four corners, placed as the mask's sibling. There is no
node-clone function to copy the sword node wholesale: the class's vtable slot 0x00 (main+0x866a4) is
not a function in Ghidra's analysis, which is why r45 faulted calling it. 22 host fixtures pass;
verify PASSED with the same 21 signature windows. Gameplay NOT RUN. The MT `color` word's byte order
is unproven, so the grey's alpha byte position is an inference.

r47 DEVELOPMENT (2026-10-31), superseded by r48 after its run:
Current state: r47 DEVELOPMENT candidate. The plan's Phase 5 copy did not appear on screen in the
r46 run because the build refused it: stage 52, with the cut, the mask, the storage growth and the
teardowns all behaving exactly as shipped. The reason is pinned in the decompile: the `Texture`
setter resolves an id by scanning `*(obj+0x64) + 0xe8`, the `cGUI` the sprite belongs to, and the
class's own constructor zeroes that word, so a standalone copy has no list to resolve in and its
texture object at +0x104 stays 0. r47 gives the copy the bar's `+0x64` and reports the resolved
texture object on the insertion line, which tells a copy that was never built apart from one that
exists. The flag word it prints there is the class constructor's own default (main+0xa4d0d8 writes
0x020a0001 at +0x54), so it distinguishes nothing about drawing. Everything else is the r46 build: allocation through the
DTI allocator's slot 0x20 with the DTI id, a zeroed 304-byte block, the standalone constructor
main+0xa4f60c, the fields the shipped Key Quest Markers badge sets, placement as the mask's sibling,
per-frame rect and atlas writes from core's `dim_shape`, no animation entry so nothing keyframes it,
additive stages 50..52. The accepted r44 cut, mask, storage and link code is untouched. 22 host
fixtures pass; verify PASSED with the same 21 signature windows. Gameplay NOT RUN. The MT `color`
word's byte order is unproven, so the flat dark grey's alpha byte position stays an inference.

r46 DEVELOPMENT (2026-10-31), superseded by r47 after its run:
Current state: r46 DEVELOPMENT candidate. r45 froze the game about five seconds into a quest, on
both launches, and the log says why: the copy was built by calling the class's vtable slot 0x00 as
its constructor, which on this class is not one, and the game faulted on a null read inside that
call before the insertion printed any line. r46 builds the same copy the way the shipped Key Quest
Markers badge is built on this exact class: the DTI allocator's slot 0x20 with (bytes, align, dti
id), a zeroed 304-byte block, and the standalone constructor main+0xa4f60c that takes the storage
in r0 and returns it; then the fields that recipe sets (marker, the bar's owner and resource,
position, the bar's authored pair, the drawable and layout-dirty bits, the texture id through its
setter, the bar's atlas rectangle, the four corner colours) instead of copying the bar's words.
The atlas rectangle is float pixels, like the bar's, where r45 wrote integers, and a failed copy
is now freed once. Nothing else about Phase 5 moved: the same placement as the mask's sibling, the
same per-frame rect and atlas writes from core's `dim_shape`, the same no-animation-entry
constancy, the same 6-word object array and 88-byte draw block, the same additive stages 50..52
that leave the accepted appearance. The accepted r44 cut, mask, storage and link code is
untouched. 22 host fixtures pass; verify PASSED with the same 21 signature windows. Gameplay NOT
RUN. The MT `color` word's byte order is still unproven, so the flat dark constant stays
byte-symmetric grey with the alpha byte written high, labelled as an inference. The research is
`work/modern-sharpness-gauge/PHASE5_RESEARCH.md`.

r45 DEVELOPMENT (2026-10-30), superseded by r46 after its run froze the game:
Current state: r45 DEVELOPMENT candidate, the plan's Phase 5 (the dimmed empty blade). r44 is
READY_TO_DISTRIBUTE and accepted on its own bytes, so this adds one thing and changes nothing on
the accepted path: a second copy of the blade sprite, allocated as a `cGUIObjTexture` through its
own DTI's MT allocator, constructed by the class's slot-0x00 constructor, given the bar's
transform, kind, owner, resource, texture id and atlas rectangle, coloured flat dark at all four
corners, and linked as the scissor mask's sibling on the row root's chain so it is drawn after the
mask's subtree and outside the scissor. Its rect and atlas `u` are written every frame at the
mask's enter handler, from `dim_shape` in core: the rect starts where the kept part ends and ends
at the bar rect's right edge, and `u` is the same fraction of the authored 88 columns, so the copy
keeps the blade's scale instead of squeezing the sprite into the empty part. It has no animation
entry, so no keyframe writes to it: that is what makes it constant and non-pulsing, and it is why
`Txt_sharp`'s own keyframes are untouched. The insertion grows the row's object array to 6 words
and the draw block to 5 objects ((5 << 4) | 8 = 88 bytes = 11 records, exactly the 11 the walker
emits with the edge glyph visible, 9 in the looping tiers). Its build is additive: stages 50..52
mean no copy, and the gauge then looks exactly as the accepted r44. 22 host fixtures pass; verify
PASSED with the same 21 signature windows. Gameplay NOT RUN. The MT `color` word's byte order is
unproven, so the flat dark constant is byte-symmetric grey with the alpha byte written high and
labelled as an inference; the facts line now prints the bar's four corner colours so a run pins
the packing next to the colour on screen. The research is
`work/modern-sharpness-gauge/PHASE5_RESEARCH.md`.

Moved out of the head on 2026-10-30, when the phases they described closed.

### r43, Phase 4 closed (2026-10-30)

Current state: Phase 4 closed on r43 (`DISTRIBUTION.json`, DEVELOPMENT). The tester's instruction was to
close the phase by pruning the spam log lines and removing the Plugin UI controls, amended on r43. The
panel is gone - no settings, so no panel - and with it the sample ring and its 320-sample budget, the
bucketing constants, the render-target scissor capture in the mask hook, and the clip and kept-row
probes: the sampler is discovery only now. What still prints is one line per gauge row read with the
insertion outcome, one per mask teardown, one each time the fill's ratio steps, one when the gauge HUD is
absent, and the row-check stage only when it changes. The cut is unchanged: the player's own sharpness on
the accepted 128..200 px track. verify.py's export list moved with the panel (no ImGui callbacks at all),
which is recorded beside the gate. 22 host fixtures pass; verify PASSED with the same 21 signature
windows. NRO `c3bf8c3a8d3f795647a4e7a2c838869bd579021d4416a6e34c8742d221d55ce2`. The closing build has
not been run.

### r42, Phase 4 acceptance in progress (2026-10-30)

Current state: Phase 4 acceptance in progress on r42 (`DISTRIBUTION.json`, DEVELOPMENT). Three of the
plan's Phase 4 scenarios are accepted by the tester on r42's bytes (2026-10-30): the cut tracks
depletion hit by hit and moves back on sharpening across a tier boundary with the native colour and
pulse intact; the glow and the edge glyph clip at the same edge as the blade with nothing glowing on
the empty side; HUD hiding/showing, quest exit/re-entry and HUD recreation are clean, and health, stamina, timer and
party member gauges are untouched. That run's log carries 83 forced lines, all `src=live`, ratios 10000
down to 7628 bp, every box on the accepted 129..201 px track within 1 px, and 78 gauge-row teardown
lines against 1 insertion. Left: a ranged weapon with no sharpness gauge, a later quest with a different
melee weapon (the box, never mid-quest), a non-English language, and the HD Project and viewport
claims. NRO
`40a45d6c75d09f8d3248af2353fb3c3f70ee6eefea05a34be6ae0c5afe9f739a`.
r25 stops a mask from clipping once its row's links are gone, the failure the r24 run showed at an
area transition; r24's run is the first where the mod's mask did not hide the gauge, so the
geometry is right and this is the lifecycle's turn. r24 writes the
transform's x scale, which the r23 run pinned as the reason the clip had no width. r23 cleared the mask's anchor
modes, a real defect (its run read `anchors=0xfffffff0`), and the width stayed 0. r22
adds the reacquire path the Forge skill's GUI reference asks for: the draw walker checks the
inserted mask and re-arms the row path when it is gone, so the next area's HUD gets its own
mask. r21 removes a box cache that made r20's clip box be written
once per row and never again, which is why the gauge was missing for the third run in a row. r20
makes a lost mask re-arm the whole row path, because a hunt crosses quest areas through loading
screens and each area's HUD is a row of its own. r19 moves
the clip box to the mask's own enter hook and adds the liveness check the r18 run proved
necessary. r18 was r17's insertion with one gate comparison
corrected, and its run is the first completed insertion this mod has had: the mask went in with
a 5-entry map and a 9-record cache, the bar, glow and edge stopped drawing, and the probe lines
showed why - the clip box came back as a zero-width strip, because the node's rectangle is the
game's layout's to compute. r17 is the insertion build, and the first of this mod's versions
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
   halfwords at **row+8**. The r17 run read that pair on the live row: 3 objects and 16
   sequences, with the published cache 192 = 16 * 12 into the allocation. So the record area is
   `(3 << 4) | 8` = **56 bytes = 7 records** (allocation 248): the unmodified tree's 5 fit with
   2 spare, and the tree with an inserted mask emits 9 = 72, 16 bytes past the end. r14 had the
   cache overflow and the map desynchronisation, and the cache objects field is the tree's node
   count, not node count + 1 - the map is the field that is one longer, its first entry being
   the controller. Grown to 4 objects the area is 72, exactly the inserted tree's 9 records, and
r18's run published exactly that (`records=9`).
8. The mask's rectangle at `+0xb0..+0xbc` is a derived value the game rewrites from the node's
   quad (position + authored size) on every layout. r18's run read it 383 times: 376 lines with
   `left == right`, top/bottom ours and the width gone, and the gauge invisible. A node the
   animation row has no entry for has width 0, so a box written earlier in the frame does not
   reach the scissor. The box has to be written in the mask's enter handler (main+0xa4ba94) in
   the call that reads it, with the authored pair written too so the game's rebuild computes the
   same box. The same run's last 90 seconds show the plugin writing into a freed mask (the row's
   scene ended, the root destructor fired zero times for the sixth session): the mask's parent
   link back to the captured root, checked in the enter hook, is the liveness proof.
9. The mask's anchor modes live in `mask+0xa0`'s low 4 bits, and they decide the rectangle's
   size. The quad builder main+0xa4b390 reads `& 3` as the y mode and `& 0xc` as the x mode:
   mode 0 spans `0..size`, 1 centres, 2 spans `-size..0`, and 3 collapses the axis to zero
   extent. The class factory sets bits 4-11 and leaves bits 0-3 as the freshly allocated block
   had them; the class has no `Anchor` property in its DTI record, and the game's own masks get
   the anchors from the row builder's resource record. So a factory-built mask keeps recycled
   anchor bits, and mode 3 on the x axis is what produced `left == right` with the height ours
   in r14, r18, r20 and r22. The enter handler's bit 1 rebuilds `+0xb0..+0xbc` from that quad
   (the DTI `Size` setter main+0xa4ba40 is "write the pair, set bit 1"), which is why writing
   the rectangle words never moved the scissor: r23 clears the modes and lets the game rebuild.
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
glow, edge at `instance+0xd8` with `tree_nodes = 3`. r17 put the insertion back with both
expansions and ran on Eden on 2026-10-24: its own gate stopped it at stage 48, the gauge drew
normally, and the log carried the sizing pair that showed the gate's rule was the wrong one.
r18 fixed that comparison, ran the same day, inserted the mask, and the gauge disappeared:
the clip box was a zero-width strip. r19 writes the box in the mask's enter hook, not in the
draw walker, and drops its addresses when the mask's links are gone. r18 ran on Eden the same
day: the insertion completed (5-entry map, 9-record cache) and the gauge disappeared with a
zero-width clip box, 376 of 383 probe lines at left == right, and the session's last 90 s of
writes into a freed mask. r19 is not run; r20 re-arms the row path on a lost mask, because
quests cross areas through loading screens and each area's HUD is its own row. r20 ran the same
day: the insertion completed again, the gauge disappeared, 206 of 206 probe lines at left ==
right, and no links-lost line - the box was written once, because the build cached the box it
had written and sharpness never moved. r21 removes that cache and is not run. r22 puts a read-only mask check in the draw walker, the callback that keeps running when a mask dies with its row, so the next area's gauge row is reacquired and inserted. r22 ran the same day: 225 of 225 probe lines at
left == right with sharpness moving, and the links-lost line fired once and re-armed correctly.
r23 cleared the anchor modes and ran the same day: the anchors read back cleared, the width
stayed 0, and the run pinned the remaining inputs (origin from the parent, authored width
multiplied by a transform x scale of 0). r24 writes that x scale and the bar-height authored
pair, and logs every input the builder reads. r24 ran the same day: the gauge drew from the
quest start (the first run where the mask did not hide it) and vanished at an area transition,
where the mask kept clipping after its links broke. r25 turns that mask off, from the mask's own
enter handler, through the class's DTI `Enable` setter. r33 ran on Eden on 2026-10-29 with the
installed bytes matching: all four forced steps show, the gauge survives sharpness changes,
area changes and quest changes, and the cut sits at each box's right edge; ratio 0 kept the
hilt and 8 px of blade, because mask_box was fed the 128 px rect while the bar draws at its
authored 88 px.
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
`prepare`: this mod ships no resources, assets or INI, and has no settings,
so no Plugin UI panel: the build exports `forge_onLoad`, `forge_onInit`,
`forge_onUpdate`, `init` and `fini`, and `verify.py` requires exactly that set.
Inputs come from `build.local.json` (`main`, `sdk`, `forge` under `inputs`,
copied from `work/shared/game-exefs/`); `build.local.example.json` documents
the keys. Outputs go to `build/modern-sharpness-gauge/candidates/VERSION/` and
scratch to `work/modern-sharpness-gauge/candidates/VERSION/`, where VERSION is
`scripts/paths.py`'s current version. `package` refuses to run for a
`READY_TO_DISTRIBUTE` spec: no distribution approval exists.

## Files

- `core/src/lib.rs` — `#![no_std]` decisions only: the local-player chain, the
  gauge read, the fill math in basis points, the gauge-HUD identity, the cut
  box from the accepted rect offsets, the node count and cache sizing the
  insertion gates on, the walker ports the fixtures pin, and the copied `Sample`
  with its 11 field names. No Forge, no FFI, no `unsafe`.
- `core/tests/behavior.rs` — host fixtures over a fake memory map.
- `src/hooks.rs` — the four Forge hooks (walker, instance draw, row root
  destructor, mask enter), their signature gate and rollback.
- `src/sampling.rs` — discovery in the walker's hot path: the slow attempt
  cadence, the captured controller and player, the gauge-present gate, and the
  capture's teardown.
- `src/gui.rs` — the row gates 21..34, the row facts, the retry cadence.
- `src/insert.rs` — the mask insertion, the map and cache growth, the cut box
  write at the mask's enter handler, and the liveness check.
- `src/memory.rs`, `src/svc.S` — mapping-checked reads and the three SVC
  wrappers Forge does not expose (query memory, GetInfo program id, process
  cache flush). Gap documented beside each use.
- `src/signatures.rs` — 32-byte windows taken from the original main NSO.
- `src/lib.rs` — metadata record, install, and the log lines outside the hooks.

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
- Inside the hook: no allocation, formatting, file access, float, or logging.
  Reads go through `memory::word`, which queries the mapping per access. Once
  the row is known a walker call costs two atomic loads and one integer compare.
- Discovery cadence: one attempt every 60 walker calls, so a few fractions of a
  second per HUD. Nothing is queued, and `FIELDS` names the copied words;
  formatting and logging happen only in `forge_onUpdate`.
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
  `1134034944` is `304.0` (`1133248512` is `280.0`, `1118830592` is `88.0`, `1134821376` is
  `328.0`, `1135607808` is `352.0`; setTextureRect writes them through a `float *`), and
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
fixes that as this row's sequence prefix of 16 * 12, so the sizing pair is at `row+8` and its
sequences half is 16; the r17 run read the pair itself (3 objects), which makes the record area
`(3 << 4) | 8` = 56 bytes = 7 records, so this section's original answer - r14's 9 records
writing past the end - stands, with the allocation at 248 rather than the 252 the `row+4` pair
suggests. The sizer uses one pair, `row+8`, for both numbers. The map fields came out as
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
   pair too, as `counts8`. r17 required `objects == count_nodes(root) + 1`; the run answered
   `counts8` = 3 objects against `tree_nodes` = 3, so that equality was the wrong rule and the
   insertion stopped at stage 48, having written nothing. The cache field **is** the node count;
   the map is the field that is one longer (its first entry is the controller). r18 compares
   `objects == count_nodes(root)` and keeps the map at `nodes + 1` words.
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
   uses for the same pair. On this row the area goes from 56 (7 records) to 72 (9), which is
   exactly the inserted tree's count.
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

Host fixtures: 20 pass, the two new ones being the cache sizing against the measured 192-byte
prefix (with the r17 run's 3 objects that is a 56-byte area = 7 records, so the inserted tree's
9 records are 16 bytes past the unmodified row, as r14 showed) and the node-count rule on both
trees.

## r17 run 2026-10-24 (Eden): the gate stopped it, and read the pair that fixed the rule

Log `work/modern-sharpness-gauge/runs/r17-eden-2026-10-24.txt` (638 lines, sha256
`096a79df51432d10a5e5c11b3a9bd7c3e6d2e80c55f6c8ae652c92bcbbf41aec`), installed bytes
`626d96f75fe9102febf30d9c09a2b38714fddcb943ca6e55465ab50c3c1f6749` = the r17 payload. 95 plugin
lines, t=9.98 to t=582.05, session closed at t=961.2 (user stop), no guest exception, no
SIGSEGV. The user's report: the gauge looked normal, no visual change - and it did, because the
insertion wrote nothing.

`mask insertion failed at stage 48`, once, at t=167.58, next to the facts line:
`counts=983044` (0x000F0004, the `row+4` pair: 4 objects, 15 sequences) and
`counts8=1048579` (0x00100003, the `row+8` sizing pair: **3 objects, 16 sequences**),
`cache=0x47fe7da0` - `cache_alloc=0x47fe7ce0` = 192 = 16 * 12, `tree_nodes=3`,
`obj0=0x47ff8590` = the controller and `obj1=0x48024ae0` = `tree_root` = the bar.

The gate was written to enforce the r14 crash rule and got the arithmetic of it wrong: the map
is `nodes + 1` words (4 for 3 nodes, its first entry the controller), the cache's objects field
is `nodes` (3). Stage 48 is the insertion's first check, before the factory call and every
allocation, so nothing was built, published, linked or freed - the gauge and the row are the
game's own, and the run is a clean read-only session: 13 samples (350/350 at tier 4 down to
281/350 at tier 3), 39 stage-21 lines while the HUD was absent (t=9.98..163.50), zero
root-destructor fires across five sessions.

That pair also settles the r14 cache question the other way from what r17's design text said:
the record area is (3 << 4) | 8 = 56 = 7 records, so the inserted tree's 9 records = 72 bytes
are 16 past the end. r14 overflowed the draw records **and** desynchronised the map; an
insertion has to grow both, which is what grow_map and grow_cache do.

## r18, built 2026-10-24: the one comparison the run corrected

`insert::run`'s gate now reads `objects != nodes` (with the same range bounds on the pair), and
`grow_map` keeps allocating `nodes + 1` words. Nothing else changed in the insertion: the mask
factory, the MT allocator for the map, the root's vtable 0x58/0x5c pair for the draw block, the
native linkers, the per-frame rectangle write and the 40..49 codes are r17's. `cache_layout`'s
row numbers, the r15-section text in PHASE2_RESEARCH.md, DEVELOPMENT's fact 5 and the r15/r16
run sections are corrected to the measured pair (3 and 16, area 56, allocation 248). The
fixtures pin (3,16) → (248,192,56), (4,16) → (264,192,72), the inserted plan's 9 records as
exactly the grown area and 16 bytes past the unmodified one, and the node-count rule on both
trees: 20 pass.

What r18 asks the run for: `scissor mask inserted` with the mask, map, entry count, cache and
capacity, then `clip mask rect` lines every ~2 seconds; if it stops, the stage 40..49 code; and
the bar's visible width as sharpness drops.

## r18 run 2026-10-24 (Eden): the insertion completed and the gauge disappeared

Log `work/modern-sharpness-gauge/runs/r18-eden-2026-10-24.txt` (sha256
`96c26b4f50bc1177695484aa06f50d05154a1af6dc981fb58791c8a3e3d43dc0`), installed bytes
`55340cfbec4fbdc176be57d3ac22356945c4f431f882eb784b362426a1e23330`. The user: "the gauge become
missing". The session ran 982 s, closed by the user's stop, zero guest exceptions, 22 samples
(350/350 tier 4 to 313/350 tier 3), 9 maxwell asserts and 1 dynarmic line as in earlier runs.

`scissor mask inserted: mask=0x48003060 map=0x48802000 entries=5 cache=0x47ffa8a0 records=9` at
t=129.78: the grown map, the grown cache and the mask all published, the first insertion this mod
has completed. Then 425 `clip mask rect` lines: `110 32 110 64` for the bar at
`110,38,238,166` (t=131.8..234.0), then `220 64 220 96` for a second HUD layout, bar at
`220,70,348,198` (t=236.0..883.0) - the box tracking the bar's rect, top and bottom correct, and
**376 of 383 sane lines with left == right**. A zero-width scissor hides everything under it,
which is the missing gauge. `flags=0x1` throughout, so bit 1 (rebuild) was clear when we wrote:
the game rebuilt the rectangle anyway, from the node's quad, and for a node with no animation
row entry that quad has width 0. From t=885.1 the mask's words are garbage (`flags=0x0`,
implausible coordinates) while the plugin kept writing to the freed block for the last 90 s of
the session, the row's root destructor having fired zero times for the sixth session.

## r19, built 2026-10-24: the box at the enter handler, and the liveness check

- Hook 4: `main+0xa4ba94`, the mask's own enter handler, the call that reads `+0xb0..+0xbc` and
  applies the scissor. `insert::enter` runs first: if `mask+0x50` is not the marker it returns
  (every other scissor mask in the game pays one word read and a compare); if it is, the box is
  computed from the bar's live rect and the live ratio, the authored pair, the position pair,
  the four rectangle words and `+0xa4 = 1` are written, and the original handler applies them.
  `apply`, the walker-entries write site, is deleted: the r18 probe lines are its refutation.
- Liveness in the same hook: the mask's parent must be the captured root, the root's child must
  be the mask, and the bar's parent must be the mask. When that fails the addresses are dropped
  and the writes stop, which is the freed-mask window r18 showed. The root-destructor hook stays
  (it is the documented teardown, six sessions of zero fires notwithstanding).
- Comment trims across `insert.rs`, `gui.rs`, `hooks.rs`, `lib.rs`: the offsets, invariants and
  caller contracts stay, the run-by-run narrative moves to this file and PHASE2_RESEARCH.md.

20 host fixtures pass (the cache/geometry arithmetic is unchanged, so the fixtures are
unchanged); verify PASSED with 21 signature windows. r19 is not run. The open question for the
run is the one r18 posed: is a rectangle written in the enter call the rectangle the scissor
gets. If it is, the bar shows at the live ratio; if the gauge is missing again, the probe lines
say whether the box was right at the moment the handler ran.

## r20, built 2026-10-24: one mask per gauge row, across the areas of a hunt

The tester's account of 2026-10-24: MHGU quests are not seamless, the map is divided into
areas, and moving between areas goes through a loading screen. Every run of this mod matches it
(PHASE2_RESEARCH.md's section on it): stage-21 stretches where the gauge HUD is absent, r18's
bar rect moving from `110,38,238,166` to `220,70,348,198` at t=236, and its tail from t=873 with
the bar's rect `0 0 0 0` and the mask's words garbage while the plugin kept writing.

So a gauge row is a per-area object, and the mask, the 5-word map and the 9-record cache belong
to one row. r19's link check noticed the death and dropped the insertion's addresses, which
stopped the writes into freed memory, and left the rest pointing at the dead row: `gui`'s
reading state (so the row facts and the insertion never ran again), the sampler's gauge
controller, and the instance capture. r20's failed check calls `hooks::teardown_gauge_row`, the
same re-arm the root destructor hook does: controller, capture, facts state and the insertion's
addresses all cleared, so the next HUD row is found, read and inserted like the first.
`insert::misses()` counts the losses and the log prints one line per change:
`inserted mask's links lost (N times)`. The root destructor hook stays, six sessions of zero
fires notwithstanding, because it names the documented teardown.

20 host fixtures pass; verify PASSED with 21 signature windows. r20 ran on Eden the same day:
the gauge missing, 206 of 206 probe lines at width 0, the cause in `insert.rs`'s box cache;
r21 removes it.

## r20 run 2026-10-24 (Eden): the box was written once, and a cache of it did the rest

Log `work/modern-sharpness-gauge/runs/r20-eden-2026-10-24.txt` (sha256
`e8d1d952ee65e204be23fce45ad914fa097fc660f7382524ed75d242935119dc`), installed bytes
`5617ee906b0756af39676a6a3bdc4a4bf3b5fb55d5b1d18fed29978d552635fb`. The user: "game ran, gauge
disappearing". 298 plugin lines, 15 samples (350/350 tier 4 to 343/350), session past t=743.

The insertion published again (`mask=0x441ca6b0 map=0x48802000 entries=5 cache=0x4863dbc0
records=9`), and 206 of 206 sane probe lines read `110 32 110 64` against the bar at
`110,38,238,166`: the same zero width as r18, the same `flags=0x1`. The enter hook's write
happened, once. `insert::enter` compared the box it was about to write with the box it had
written and returned early when they matched; sharpness never moved, so the box never changed,
so the authored pair, the position pair and the rectangle were written on the row's first enter
call and skipped for the rest of the session, while the game rewrote its derived rectangle in
between. A cache of a value the game owns on every frame is the bug, not a micro-optimisation:
r21 writes on every enter, five words for the one mask, per frame.

Two things this run also settles:

- the link check never reported anything, and it cannot in this case. From t=630 the probe reads
  freed memory (`flags=0xff7f7f7f`, words that are the float 1.0, implausible coordinates) with
  the mask and bar addresses unchanged. Once the mask dies with its row, the enter handler stops
  being called, so the handler's liveness check stops running: no writes into freed memory (the
  r18 90-second window is gone, which is what the check bought), and no re-arm either. The
  plugin sits out the rest of the session and the gauge draws as the game builds it. A re-arm
  for that case needs a signal that does not depend on the dead object; not built.
- the row's objects live from t~215 to t~630 in one session, which is consistent with the area
  loading screens the tester described; the bar's rect never moved to a second layout this time.

## r21, built 2026-10-24: write on every enter

`insert::enter` writes the authored pair, the position pair, the four rectangle words and the
enable bit on every call, no remembered box. Nothing else changed: the marker filter, the link
check, the geometry from the bar's live rect and the live ratio, and the re-arm stay.

20 host fixtures pass; verify PASSED with 21 signature windows. r21 is not run. The open
question stays open: whether the rectangle written in the enter call is the rectangle the
scissor keeps, with the writes now happening every frame.

## r22, built 2026-10-24: reacquire the row after an area's loading screen

The Forge skill's native GUI reference was updated on 2026-10-24 with the tester's account, and
two of its rules govern this insertion (PHASE2_RESEARCH.md quotes them):

- **Update point.** It asks for a write after the final native write and before transform/cache
  construction. The mask's rectangle is derived from the node's quad, so every pre-transform
  write is overwritten - r18 (walker entries) and r20 (enter call, cached) both ended with 206+
  probe lines at left == right. The enter handler is where the game consumes the rectangle;
  writing the authored pair and position with it makes the next transform compute the same box.
  That is the site the evidence leaves open, and r21 made the writes happen every enter.
- **Reacquire.** It asks how the plugin reacquires new objects without retaining stale pointers,
  and to include recreation in validation. r20/r21 dropped the addresses inside the mask's enter
  handler, which cannot report a mask freed with its row: the handler stops being called (r20:
  freed-object reads from t=630, no links-lost line, the plugin idle for the rest of the
  session). r22 adds `insert::check_liveness` to the draw walker's callback - the marker at
  `mask+0x50`, the mask's parent against the captured root, the root's child against the mask -
  and on failure it counts the loss and runs `hooks::teardown_gauge_row`, so discovery restarts
  and the next area's row is read and inserted like the first. The check is read-only: the mask
  is reached through captured links, so a stale read can only drop addresses, never write into
  freed memory. Three word reads per walker call, only while a mask is inserted.

20 host fixtures pass; verify PASSED with 21 signature windows. r22 is not run: the clip
geometry (r21's) and the reacquire (r22's) both need a run, and recreation across areas is now
part of what a run has to cover.

## r22 run 2026-10-24 (Eden): 225 of 225 zero-width, and the reacquire proved working

Log `work/modern-sharpness-gauge/runs/r22-eden-2026-10-24.txt` (sha256
`11215d07f141c1e313560e4bd9c367b644078681549d0c3a96a93526eeb13fa6`), installed bytes
`a18290a6df0b2cbe2edaf67e49a612a16540306f41c8cd38544c570729a69945`. The user: the gauge
disappeared from the start of the quest and never recovered. 359 plugin lines to t=714.5.

- `scissor mask inserted: mask=0x48770db0 map=0x48c02000 entries=5 cache=0x48780d50 records=9`
  at t=139.89; 14 samples, sharpness 350/350 tier 4 down to 328 tier 3, so the clip box changed
  during the session.
- 225 of 225 sane probe lines have width 0: `110 32 110 64` (bar at `110,38,238,166`) and
  `660 192 660 224` (bar at `660,198,788,326`), `flags=0x1`. r21's every-frame write was in
  effect, so the write site is not the reason the scissor is empty.
- `inserted mask's links lost (1 times)` at t=591.98: the walker check fired, dropped the
  addresses and re-armed, and after it the log carries stage-21 lines to t=712 (no gauge HUD to
  find: the quest had ended). This is the first run where that path proved it works.

## r23, built 2026-10-24: the anchor modes the factory leaves alone

`insert::enter` now writes `mask+0xa0 & !0xf` - both anchor modes 0, the quad spanning
`0..size` on each axis - and `+0xa4 = 3`, which asks the game's own enter handler to rebuild the
rectangle from the authored pair and position written in the same call, then apply it. The
rectangle words are no longer written: main+0xa4b390 and main+0xa4ba94 say they are derived
state, and the row's layout raises bit 1 every frame, so the game recomputes them from the quad.

The static chain, in PHASE2_RESEARCH.md's section on it: the builder's mode table, the factory's
`(word & 0xfffff000) | 0xff0`, the DTI's property list (no `Anchor`), and the four runs' shared
signature (left == right, top = position, bottom = position + the height written). The probe
prints the anchor word, so a run can read the modes back next to the rectangle.

20 host fixtures pass; verify PASSED with 21 signature windows. r23 is not run.

## r23 run 2026-10-24 (Eden) and r24, built 2026-10-24: the width is the transform's x row

Log `work/modern-sharpness-gauge/runs/r23-eden-2026-10-24.txt` (sha256
`c1430d156508031d883b5d572a3b73dad597c1615d1bc57a93f785f607896ba0`), installed bytes
`909dd7fa281cc8863c79fef6d91b49cd5ecd2da837ee30ad6d5ed01f7befc225`. The user: "still disappeared
at start". 76 plugin lines, insertion at t=159, 14 clip lines, 5 samples.

`clip mask rect=110 38 110 70 flags=0x1 anchors=0xfffffff0 bar rect=110 38 238 166`: the anchor
clear landed (low nibble 0) and the width is still 0. The rect's origin is `110 38`, the row
root's own origin (the tree line reads `own=110 38 238 166` for the root, `116 32 244 160` for
the bar) while the plugin wrote position `110, 32` - the `PosX`/`PosY` DTI properties reach the
matrix only through the row's layout, which has no entry for this node. The authored pair
contributed its height (32) and no width, which against main+0xa4b390's composition
(`x = (+0x40) + (+0x20)*y + (+0x10)*x + (+0x30)`) means the width row `+0x10/+0x14` is (0, 0)
and the height row is (0, 1): the factory's matrix template has no x scale.

r24 writes `+0x10 = 1.0f`, `+0x14 = 0`, the authored pair (ratio width, the bar's rect height,
so the clip spans the bar top to bottom), keeps the anchor clear, and sets `+0xa4 = 3`; the
position writes are gone as inert. The probe logs the builder's inputs and the bar's same fields
next to the rectangle, so the next run reads each input back rather than inferring it.

20 host fixtures pass; verify PASSED with 21 signature windows. r24 is not run.

## r24 run 2026-10-24 (Eden) and r25, built 2026-10-24: the gauge draws, and a stale mask hides it

Log `work/modern-sharpness-gauge/runs/r24-eden-2026-10-24.txt` (sha256
`97ef795e301a30fce7cbeaefcf3b14c55cc7878d04b38ad13d98a1fd92656e7d`), installed bytes
`6a16c5429a15e761db5b4168586343dfa4ca59e19fdb74143fc00cfd1b74994b`. The user: shown at quest
start, then disappeared on area changes. Insertion at t=91.17, 29 clip lines to t=147.2,
`inserted mask's links lost (1 times)` at t=149.89, stage-21 lines to the end at t=162.0.

Every r24 write read back: `anchors=0xff0`, `mat` starting `0x3f800000` (the x scale), the
rectangle `110 38 110 166` with the bottom on the bar's own bottom. The authored width reads 0
between frames (`size=0,128`) because the row's layout zeroes it each frame; the width the
scissor receives is the one written in the mask's enter call, which is where the plugin writes it.

The disappearance is the mask outliving the plugin: at the transition the links broke, the
walker's check dropped the addresses and stopped writing, and the mask went on applying its last
rectangle, hiding the gauge in the new area. r25 disables the scissor at that point, from the
mask's own enter handler where the game has handed over a live object, through
`main+0xa4ba20` - the class's DTI `Enable` setter, read from the NSO bytes as
`ldr r2,[r0,#0xa4]; bfc r2,#0,#1; bfc; str; bx lr`: it clears the enable bits and ignores the
second argument. The walker's check stays read-only, as the rules require.

20 host fixtures pass; verify PASSED with 21 signature windows. r25 is not run.

## r33, built 2026-10-29: the plan's fixed ratios, where the eye can see each step

r32 (NRO `7a551d24e91e5c888ece19189d57505c5fb9acf0d2d17c6505439486feed97f1`) was packaged and
never run, so r33 replaces it and its candidate folder stays as it is. r33 starts from r31's
behaviour, the last build a user ran. Two changes, both in `src/insert.rs`:

1. The glow write in `run()` is gone, with the `NODE_VISIBLE` import it needed. The insertion
   ends at the root's sibling write, exactly as r31's did. The glow's flags are untouched, so the
   sampler counts it visible again and the mask clips it along with the bar and the edge.
2. `enter` feeds `mask_box` the plan's `prototype_ratio` (0, 2500, 5000, 10000 bp, 20 s each)
   from a counter `forge_onUpdate` ticks. 60 updates count as one second, the same divider as
   lib.rs's 120-update clip-probe log, so the step length follows the frame rate and not a clock:
   the shortcut is named beside the constant. The live ratio is still read and computed and is
   unused, and the row path, the gates, the insertion order, the links and the mask's own writes
   are unchanged. Nothing was deleted.

The ratio-change line is queued inside the enter hook when the forced ratio steps and printed by
`forge_onUpdate`: ratio_bp, the box, the bar's own rect at `+0xe0`, the effective rect at `+0xf0`
and the atlas rect at `+0x10c`. Those are core's `NODE_RECT`, `NODE_EFFECTIVE_RECT` and
`NODE_TEXTURE_RECT`, which the sampler already walks, so no offset and no field read is new, and
no formatting happens in the hook.

21 host fixtures pass. The new fixture feeds `mask_box` the four ratios at the bar's live 128 px
rect: the boxes are `[110, 32, 26, 32]`, `[110, 32, 48, 32]`, `[110, 32, 71, 32]` and
`[110, 32, 116, 32]`, the cycle repeats after four steps, and the empty step keeps a 26 px box,
so the degenerate zero-size scissor is never asked for. Verify PASSED with the same 21 signature
windows, the same exports and the same hook window as r31. NRO
`ed5cb4451594f43fef10f75e6967ac3a2dc1c29ef7fa6193aba097d20337f9c4`. Gameplay NOT RUN.

## r33 run 2026-10-29 (Eden): all four steps show, and the box width is the last offset

Log `work/modern-sharpness-gauge/runs/r33-eden-2026-10-29.txt` (sha256
`4e770bcfc3a00885acc765a5349b17768650893c8a9a48e3cc9a22459f084ba7`), installed NRO
`ed5cb4451594f43fef10f75e6967ac3a2dc1c29ef7fa6193aba097d20337f9c4`, matching the r33 payload.
1598 plugin lines, 2 insertions, 1 links-lost recovery, 0 insertion failures, 617 clip lines.
The tester: the four steps are visible, and the gauge survives sharpness changes, area changes
and quest changes; "there are no empty gauge", and "I expect 0% means no blue blade parts".

Every step's box reached the screen: the forced lines wrote 26, 48, 71 and 116 px wide boxes,
the clip lines built them as `rect=110 38 136 166`, `158`, `181` and `226`, and measuring the
four screenshots gives blue ink up to preview columns 45, 77, 108 and 135, i.e. 141, 163, 184
and 202 GUI px on the preview scale. Measuring the bar sprite's ink in the decoded atlas
(`work/modern-sharpness-gauge/visual/hud00.png`, frames at rows 280/304/328) puts it at
128..189 px, so each step's cut sits at its box's right edge, 3..9 px into the blade.

Two refuted assumptions, both from the run:
1. `bar_width = rect[2] - rect[0]` (128). The bar's authored pair is 88x24 (`bar_w=88` in the
   facts line) with transform x scale 1.0 (`mat=0x3f800000` in every clip line), so the sprite
   draws 88 px from the rect's left and its ink columns are pixels 1:1. r33's feed scaled the
   inset to 26 and the track to 90, which is 1.4545x too wide.
2. The track starting at the ink's first column (18). Inside 18..79 the first 10 columns are
   the hilt square, and the blade is 28..79. At ratio 0 the box (110..136) kept the hilt and
   8 px of blade; the tester's 0% wants the hilt with no blade.

The `eff=` fields read `0 0 0 0` in every forced line: at the mask's enter call the bar's
effective rect is not written yet, so that part of the line measures nothing. The `tex=` words
are the bar's atlas rect as float bits, 0, 280, 88, 304.

## r34, built 2026-10-29: the box from the bar's authored width, the track from its blade

`enter` now reads the bar's authored width at `bar+0xb4` (the field gui.rs's stage 32 gate
accepts, printed as `bar_w`) and feeds that to `mask_box`; the rect's 128 px is no longer the
width, and the forced line prints the width used as `bar_w=`. Core's track is the blade:
`BAR_INK_LEFT` 18 stays the box's left, `BAR_HILT_COLUMNS` 10 is new, `BAR_INK_TRACK` is 52.
Steps at the authored 88 px: boxes 28, 41, 54 and 80 wide, i.e. 138, 151, 164 and 190 px, which
keeps the hilt at 0% and covers the blade's last column at 100%. Nothing else moved: the gates,
the insertion order, the links, the mask's own writes and the glow stay as r33 ran them.

21 host fixtures pass (the geometry fixtures now carry the measured hilt and blade columns, and
`the_forced_ratios_step_the_box_from_the_bar_authored_width` pins the four steps and refutes the
128 px feed). Verify PASSED with the same 21 signature windows. NRO
`d27c1b25efebf55f358898cc889b7d3f3db5d79623585919926b1117da111a75`. Gameplay NOT RUN.

## r34 run 2026-10-29 (Eden): the authored-88 feed cuts the tip

Log `work/modern-sharpness-gauge/runs/r34-eden-2026-10-29.txt` (sha256
`7fff76baf2e1481968dd5eb4185bc00ce2c2f14991d6c0ad1178f279ba60ad4f`), installed NRO
`d27c1b25efebf55f358898cc889b7d3f3db5d79623585919926b1117da111a75`, matching the r34 payload.
521 plugin lines, 31 forced-ratio lines, 1 insertion, no links-lost, no insertion failure. The
forced lines read `bar_w=88`, so the authored pair does read 88 inside the mask's enter call, and
the boxes were 28, 41, 54 and 80 px (rights 138, 151, 164, 190).

The tester: "those fills are definitely wrong, and worse than r33; the 0% of r34 looks like no
change from r33 at all, and r34's 100% has a weird cut at the end, unlike r33's 100%". Measuring
the four screenshots with the logged box rights as the ruler puts the cuts at 138, 151, 164 and
190, i.e. exactly on the boxes, which is why the 100% step cuts the blade near the tip. Two
refutations:
1. The authored 88 px is not the drawn width. r33's 100% box (right 226) shows the whole blade;
   r34's (190) does not. The node's rect width (128) is the draw scale, as r33 had it.
2. r34's hilt column split does not fix the empty step. The hilt glyph's left edge measures
   ~125 px, so a box ending at 136 (r33) and one at 138 (r34) both keep it, and the two 0%
   screenshots look the same. The empty box has to end left of 125.

The atlas column split from the r33 reading (hilt 18..27, blade 28..79) is kept as a measurement
of the art, not as the clip's geometry: the clip's numbers now come from the two photographs.

## r35, built 2026-10-29: the box from the rect width, the empty step left of the hilt

`enter` feeds `mask_box` the rect width again, and core's track is measured from the photographs:
`BAR_INK_LEFT` 10 and `BAR_INK_TRACK` 70 columns, i.e. at the 128 px rect 14 px in and 101 px
long. Steps: 124, 149, 174 and 225 px. At 124 the box covers 110..124, left of the hilt glyph, so
the 0% step draws nothing of the bar, the glow or the edge; 225 matches the r33 box (226) that
photographed the whole blade. The forced line prints both widths, `bar_w=` (the authored read) and
`feed=` (what the box used), so the next run shows the mapping without a screenshot.

21 host fixtures pass (the geometry fixtures now assert both feeds: the rect width at 124/149/174/
225, and that the authored 88 feed's 100% box ends at 190, inside the blade). Verify PASSED with
the same 21 signature windows. NRO `21413972b0f2ce8c6db01e8de5cdc5b9cab1d062e5e830237e7f89c2acaaf482`.
Gameplay NOT RUN.

## r35 run 2026-10-29 (Eden): the ends are right, the middle steps are not

Log `work/modern-sharpness-gauge/runs/r35-eden-2026-10-29.txt` (sha256
`963a5b388507ad70b6b552f5d52ba06ec736b27758f84ef1277e0666e54641f6`), installed NRO
`21413972b0f2ce8c6db01e8de5cdc5b9cab1d062e5e830237e7f89c2acaaf482`, matching the r35 payload.
61 forced lines, `feed=128` and `bar_w=88` on every one, boxes 14/39/64/115 (rights 124, 149, 174,
225), 1 insertion. The tester: 0% is good and 100% is full, the 25% and 50% positions look wrong,
and the 100% fill may already be past the gauge length.

Measuring the four screenshots with the logged rights 149 and 174 as the ruler (1.4 image px per
GUI px): the 0% step has no blue pixels at all; the 25% step's blue runs 130..149; the 50% step's
130..174; the 100% step's 124..**200**, i.e. the drawn gauge's ink ends at 200 px while the box
ran to 225. So the visible blade was 0, 19, 44 and 70 px of a 70 px blade: the steps read 0, 27,
63 and 100%, which is the tester's report, stated as arithmetic.

Refuted: the track's outer end. 225 is the 128 px rect's own edge, not the gauge's: the drawn
gauge is 124..200 (76 px), and a track of 111 px (124..235) spread the four steps over 1.45x the
gauge's length. r33's 100% box (226) looked right only because it was past the tip.

## r36, built 2026-10-29: the track ends at the blade's tip

One constant: `BAR_INK_TRACK` 70 -> 55 columns, so at the rect's 128 px the track is 80 px long
and the four steps end at 124, 144, 164 and 204 px. The 204 is the measured tip (200) with margin,
measured with the r35 log's cuts as the ruler. Everything else is r35's: the rect-width feed, the
gates, the insertion order, the links, the mask's writes, the glow, the forced-ratio clock and the
log line.

21 host fixtures pass (the geometry fixtures now assert the 124/144/164/204 steps, that the 100%
box covers 200 px and stays under 210, and that the authored 88 px feed ends at 190, which
photographed as the cut tip). Verify PASSED with the same 21 signature windows. NRO
`52ecec7b87561293fc44f992d94e777ff549f165775d0a733acdd127cc888c3f`. Gameplay NOT RUN.

## r37, built 2026-10-29: a 0-100 slider, so a value can be set on demand

The tester asked for a better test than the 20 s cycle: a Forge option, a slider 0-100 that
changes how much of the gauge shows. `panel()` opens one closed tree row
(`Modern Sharpness Gauge##modern-sharpness-gauge`, TreePop paired through Drop) with
`Gauge shown %`, 0..100 clamped. Dragging stores basis points in one atomic; the mask's enter
handler reads it in place of the cycle, so the box changes the next frame. Untouched
(`u32::MAX`) keeps the cycle, which is how the previous runs' evidence stays comparable. The
forced line adds `src=cycle|slider`, rate-limited to one line per 30 updates so dragging cannot
flood the log.

Session-only on purpose: no `forge_config_create`, no save-data mount, no INI. `verify.py`'s
forbidden-import list was narrowed for exactly that reason (the tree row and slider imports are
now expected; config creation, save-data mount and the colour editor are still refused) and the
comment beside it says so.

r36's measurement is in this build too: `BAR_INK_TRACK` 55 columns (80 px at the 128 px rect),
steps 124, 144, 164 and 204 px against the measured 124..200 px gauge. 21 host fixtures pass.
NRO `3361d321f08340aaf02bd82938c112d02e491bd58fb8099921f6073bf1f17c9b`. Gameplay NOT RUN.

## r38, built 2026-10-29: the endpoints become sliders

"Add option to pixel start-end for 0-100% settings": the numbers we kept measuring after each run
(124..200 px from the r35 photographs) are now input fields. Two more sliders in the same Plugin UI
row, `Cut start px` 111..237 and `Cut end px` 113..238, absolute screen px on the bar's rect.
core's `span_box(rect, ratio_bp, start, end)` puts the box's right edge at `start` at 0% and at
`end` at 100%, linear in between, width never below 1 and never past the rect. `enter` uses it once
both sliders are set and core's `mask_box` track while they are not, so an untouched panel is
r37's behaviour.

The endpoint math is in core, so the fixture tests it: the defaults (14 and 94 px from the rect's
left) give 124/144/164/204 px, match `mask_box` at the 128 px rect, shift by exactly the pixels the
tester drags, and stay inside the rect. 22 fixtures pass. NRO `dd0d9a6bd7b556adbbd686a96b8d2c36cc7b9a0f6fe6419d55490db402650df0`. Gameplay NOT RUN.

## r39, built 2026-10-29: a draft, an Apply and a Reset

"Please add apply and reset button", and the report that r38's sliders were not applied directly.
Reading the r38 code, the cause is a gate I wrote: `enter` used the custom endpoints only if both
were stored, and an untouched slider stores nothing, so dragging one endpoint kept the box on
core's track and the drag did nothing visible.

r39 changes the shape of the control instead of patching that gate. The three sliders write
draft values; `apply()` publishes percent, start and end as one set and marks the panel live, and
`enter` then uses `span_box` with all three. Before the first Apply the panel is inert and core's
measured track plus the 20 s cycle drive the box, so the earlier runs' evidence stays comparable.
`reset()` clears the draft, the live set and the live mark. The row shows `Applied: 40.0% cut
124.0..204.0 px` or `Not applied: cycle on the measured track. Draft ...` so there is no guessing
which state the gauge is in. The two pixel sliders move in 0.1 px steps: the raw
`igSliderFloat` binding is used with the `%.1f px` format (forge::imgui::slider_float hardcodes
ImGui's default), and `set_draft_cut_start/end` snap the stored value to the same grid with integer
centi-pixels, because this crate is no_std and has no `f32::round`.

22 host fixtures pass (the endpoint fixture from r38 still pins `span_box`'s linearity and
bounds); verify PASSED with the same 21 signature windows, imports now also `igButton` and
`igSameLine`. NRO `40459611a838b562018772d9530296cabf6e03a20988a1645e4703d5ea91c4de`. Gameplay NOT RUN.

## r39 run and r40, 2026-10-29: the accepted endpoints become the defaults

The tester ran r39 on Eden. Installed NRO `40459611a838b562018772d9530296cabf6e03a20988a1645e4703d5ea91c4de`,
matching the packaged payload. Log copy `work/modern-sharpness-gauge/runs/r39-eden-2026-10-29.txt`
(sha256 `55c332f5c129dc63afb8d4174d554a525b32f255c48833eb7ec7b9c5daf81878`), 1125 lines, 563 of them
ours, 14 forced, no insertion failure.

The panel path works. Every forced line in that run reads `src=slider`, so the overlay thread's write
reached the enter handler, and the box's right edge is the published endpoint at both ends:

| cut (start, end px) | ratio_bp | box width | right edge |
| --- | --- | --- | --- |
| 124 / 204 | 0 | 14 | 124 |
| 124 / 204 | 10000 | 94 | 204 |
| 126 / 201 | 0 | 16 | 126 |
| 126 / 201 | 1938 | 30 | 140 |
| 126 / 201 | 2881 | 37 | 147 |
| 126 / 201 | 4871 | 52 | 162 |
| 126 / 201 | 10000 | 91 | 201 |
| 129 / 201 | 0 | 19 | 129 |
| 129 / 201 | 10000 | 91 | 201 |

`126 + 75 * ratio_bp / 10000` predicts 140.5, 147.6 and 162.5 for those three middle ratios, and the
run logged 140, 147 and 162: the floor of the same line. So the endpoints land where the panel puts
them, and the fill is linear between them, which is what r35 and r37 were measured to get wrong.

One cosmetic gap: the log prints the endpoints as raw float bits (`cut=1123811328 1128857600` is 122.0
and 200.0) because no_std has no float formatting. The panel's readout line is the readable form.

"Okay, now we got a good start/end, which is 129 as start, and 201 as end." r40 makes that pair the
defaults: `CUT_START_DEFAULT` 129.0 and `CUT_END_DEFAULT` 201.0, so the sliders open there, Apply
publishes them, and Reset returns to them. The endpoint fixture now pins that pair (rights
129/147/165/183/201 at 0/25/50/75/100%, 18 px per quarter) instead of the measured track's
124/144/164/204, which core's untouched cycle still uses through `mask_box`. The slider range
111..237 px, gates, insertion order, links, the mask's writes and the glow are unchanged.

22 host fixtures pass; verify PASSED with the same 21 signature windows. NRO
`3d5623a7a99b844b8dd22edb69e375a262529c46d1ad6041c8a21e7ca476dce5`, ZIP
`build/modern-sharpness-gauge/candidates/r40/delivery/modern-sharpness-gauge-r40-candidate.zip`
sha256 `69834c41be65da238db751a284b1c16763031933e690ed91cb27c851ca9b8eff`. Gameplay NOT RUN: the
defaults themselves have not been seen on screen, only the same numbers applied by hand in r39.

## r40 and r41, built 2026-10-29: the accepted track becomes the build's own

r40: two constants. `CUT_START_DEFAULT` 124.0 → 129.0 and `CUT_END_DEFAULT` 204.0 → 201.0, the pair
the tester dialled in and accepted on r39. The endpoint fixture moves to that pair (rights
129/147/165/183/201 at 0/25/50/75/100%, 18 px per quarter) and keeps the shift-by-20 and bounds
checks. NRO `3d5623a7a99b844b8dd22edb69e375a262529c46d1ad6041c8a21e7ca476dce5`.

"why you build this? just continue the development phase" — so r41 does the phase's actual work
instead of shipping defaults nobody has to touch. `enter` had two box paths: `mask_box` with core's
measured track while the panel was untouched, and `span_box` with the panel's endpoints after Apply.
The accepted geometry is 129..201, so the untouched path was the one cutting the older 124..204.
r41 deletes that branch: the box is built from `CUT_START`/`CUT_END` in every state, so the 20 s
cycle cuts 129, 147, 165 and 201 px, and `mask_box` leaves the runtime path (it stays in core, where
the fixtures use it to record that the authored 88 px feed is refuted). `APPLIED` now gates only the
ratio source: Apply publishes `Gauge shown %`, Reset hands the ratio back to the cycle.

22 host fixtures pass; verify PASSED with the same 21 signature windows. NRO
`2f14773db943988282a8a6a4bc748610190308f9f1d22142275290ae3b61e579`, ZIP
`build/modern-sharpness-gauge/candidates/r41/delivery/modern-sharpness-gauge-r41-candidate.zip`
sha256 `e996f73556d88a41aada2f6ae6d3cac2f55849348b6afcc00fc609b6d93034b3`. Gameplay NOT RUN: what the
cycle lands on is measured from the r39 log's applied pair, not seen on screen in r41.

## r41 run and r42, 2026-10-29: Phase 2 closes, the gauge follows your sharpness

The tester ran r41 on Eden. Installed NRO `2f14773db943988282a8a6a4bc748610190308f9f1d22142275290ae3b61e579`
= the packaged payload. Log copy `work/modern-sharpness-gauge/runs/r41-eden-2026-10-29.txt` (sha256
`6583fa69d8e19b620d2820780d316da838ec8243e494b2a29872075fcc5b0c8f`), 1125 lines, 170 ours: 6 cycle lines
at rights 129, 147, 165 and 201 px, 2 slider lines (25% and 50% published), no insertion failure. Four
crops and the two panel shots are in `work/modern-sharpness-gauge/visual/r41/`.

"its right, looks right". Measured, that report is exact: the blue ink's right edge in the four crops is
34, 63, 89 and 142 image px against box rights 129, 147, 165 and 201 px, and `34 + 1.5 * (box_right -
129)` reproduces all four within 1-2 image px (under 1.4 GUI px). 0% leaves only the hilt glyph, 100%
covers the tip with the box sitting past it, as in r35's measurement. That is the plan's Phase 2 exit
evidence on the accepted track: anchored at the hilt, linear, empty side clear, tip covered.

r42 is Phase 3's functional candidate. The fixed-ratio cycle is gone, so `enter` drives the box from the
player's own sharpness: `sampling::live_gauge()` reads the captured local player's sharpness points
through the same checked mapping Phase 1 verified, `basis_points` clamps them, and `span_box` puts the
result on the accepted 129..201 px track. Two things follow from that and are both plan rules, not new
invention: an unreadable player or a non-positive maximum is unsupported state, and the clip is turned
off there (`ENABLE_SETTER` with 0) so the native gauge draws as the game draws it; and the captured
player pointer is dropped with the controller in `clear_controller`, which is the stale capture the
plan's Phase 2 blocker named. The prototype clock is gone with the cycle; `UPDATES` stays only to rate
limit the log line.

22 host fixtures pass; verify PASSED with the same 21 signature windows. NRO
`40a45d6c75d09f8d3248af2353fb3c3f70ee6eefea05a34be6ae0c5afe9f739a`. Gameplay NOT RUN: the per-frame fill
has not been seen on screen, only the fields, the clamping and the endpoints, each proven on its own.

## r42 run, 2026-10-30: the live fill, the glow and the lifecycle are accepted

The tester ran r42 on Eden. Installed NRO `40a45d6c75d09f8d3248af2353fb3c3f70ee6eefea05a34be6ae0c5afe9f739a`
= the packaged payload. Log copy `work/modern-sharpness-gauge/runs/r42-eden-2026-10-30.txt` (sha256
`bd47e19b34ec2e4d6f2a8ce5ce8231ebff30ec328c6e09a1d65728354a1377ba`), 708 lines of ours.

"Those three tested well, and working": the hunt (depletion hit by hit, back on sharpening, across a
tier boundary with native colour and pulse), the glow and edge glyph clipped at the same edge with
nothing glowing on the empty side, HUD hiding/showing plus quest exit/re-entry and HUD recreation, and
health, stamina, timer and party member gauges untouched.

The log says the same thing in numbers. All 83 forced lines are `src=live`, so no slider was touched:
ratios run 10000 down to 7628 bp and box rights from 201 to 183 px, and every one sits within 1 px of
`129 + 72 * ratio / 10000`, i.e. the live fill is linear on the accepted track. 28 sample lines read
the player chain (350/350, tier 4, hunter 0, cmd_tbl_id 1000) with the gauge identity tuple intact.
Lifecycle: 1 insertion, 78 dropped-gauge-row lines and 1 inserted-mask-links-lost line across the
scene changes, with no second insertion, and 2 lines report the HUD absent with the clip off.

That closes the fill, the geometry and the mask lifecycle on r42's bytes. The remaining Phase 4 items
are listed in `work/modern-sharpness-gauge/PLAN.md`. No version bump: the accepted bytes are the
bytes in `DISTRIBUTION.json`.

## r42 run 2 and r43, 2026-10-30: weapon variety, a no-gauge weapon, a non-English game, 128..200 px

The tester ran r42 over three quests - Long Sword, Dual Blades, then a bowgun - with the HD Project mod
off and a non-English game. "All right." Log copy
`work/modern-sharpness-gauge/runs/r42-eden-2026-10-30-run2.txt` (sha256
`2035577d30286ba32fe04eaaf588300a2357be17dc9a19c11a06fc8a55ab6307`), 2902 lines of ours: 148 forced
lines, 145 of them `src=live`, 42 sample lines, 3 gauge-row insertions with their teardowns.

The sample lines carry the weapon variety in the fields: cmd_tbl_id 1000 (Long Sword), 2000..2033 (Dual
Blades) and 9999 (no weapon command, the bowgun), with the gauge identity tuple - anim 4, 4 objects, 15
sequences, first_object 29 - unchanged across all of it. That is the structural lookup holding in a
non-English game, which is what Phase 4 asked for. Depletion reaches current 8 and then 0 at tier 0, and
the box holds its 0% width there.

Their measurement from this run: 0% should be 128 px and 100% should be 200 px. Three `src=slider` lines
show them dragging the start to 128.0, box right 128 at 0%. So r43 moves `CUT_START_DEFAULT` to 128.0 and
`CUT_END_DEFAULT` to 200.0: 1 px tighter at both ends, which puts 0% 3 px past the ink start (measured 124)
and 100% exactly on the measured tip (200). The quarter steps become 128, 146, 164 and 200 px. No other
change: the live fill, the gates, the insertion and links, the glow, the panel and the session-only rule
are the bytes they ran.

22 host fixtures pass (the endpoint fixture now pins 128/146/164/182/200 px at 0/25/50/75/100%); verify
PASSED with the same 21 signature windows. NRO `58bfecdae7782b156314ee73e87fbcf7efaaf3c3d8c52e15da084cb8ceea16b6`,
ZIP sha256 `7bed54b2ba7c5a9172ac31b284e891efd2879de56047a4c1f858c4404013bc99`. r43 gameplay NOT RUN.

## r43 closing, 2026-10-30: the panel out, the spam out

"Close phase 4 by pruning spam logs and remove the forge imgui options to set sharpness, start, end,
apply and reset, amend them as r43."

Removed: `forge_onImGuiInit` and the panel row (Gauge shown %, Cut start px, Cut end px, Apply, Reset),
`SLIDER_BP`, `APPLIED`, the draft atomics and their setters, `live_state`, and the endpoint atomics -
`CUT_START_DEFAULT` 128.0 and `CUT_END_DEFAULT` 200.0 are plain constants now. `enter` takes its ratio
from the player's sharpness alone.

Removed log sites and the code that only fed them: the sample drain loop with `sample_line` and
`node_line` (42 and 162 lines in the last run), the clip readback block (680 lines) with `insert::probe`
and the render-target scissor capture in the mask hook (5 word reads per frame for a line printed twice
a second), the dropped-row probe block (1538 lines) with `probe_kept`/`kept`/`KEEP`, and the
320-sample ring, its budget line and the bucketing constants. The row-check stage now prints on change,
not per retry. Kept: the gauge-row facts and insertion outcome, the mask teardown lines, the fill
ratio line (19 words, endpoints and source dropped because they are constants now), and the HUD-absent
line.

One gate moved, and it is mine to move: verify.py asserted the export set including
`forge_onImGuiInit`/`forge_onImGuiRender`. With no panel those callbacks do not exist, so the gate now
requires `forge_onLoad`, `forge_onInit`, `forge_onUpdate`, `init`, `fini` and nothing else - the same
strict equality, a smaller set. Nothing about the hook window, the signatures, the payload inventory or
the flag audit changed.

22 host fixtures pass; verify PASSED. NRO `c3bf8c3a8d3f795647a4e7a2c838869bd579021d4416a6e34c8742d221d55ce2`,
ZIP sha256 `fda46586ab95dbb06485b0d562ebdb95666d6ed12fe52aa7b909bb6bd434e81a`. The closing build has not
been run: the accepted behaviour is on r42's bytes and the 128/200 endpoints are the tester's measurement.

## r44, built 2026-10-30: what the accepted build no longer needs

The tester accepted r42's behaviour and closed Phase 4 with the r43 instruction,
so r44 is forge-modding P7: remove the scaffolding, keep the accepted cut, and
fix the wording that still described experiments. No hook, signature, gate,
insertion order, link, mask write, glow or `svc.S` changed.

Removed, with what each one was for:

- core's measured ink track: `mask_box`, `boundary`, `kept`, `BAR_INK_LEFT` 10
  and `BAR_INK_TRACK` 55 sprite columns against `SPRITE_WIDTH` 88. Fed the rect
  width it gave the 124 / 144 / 164 / 204 px steps; fed the bar's authored 88 px
  its 100% box ended at 190 px and cut the blade near the tip. That track was the
  box builder while the Plugin UI was untouched; the accepted endpoints replaced it
  in r41, and r44 deletes the dead builder with it.
- `PROTOTYPE_RATIOS` and `PROTOTYPE_SECONDS`, the plan's fixed 0 / 25% / 50% /
  100% cycle, and `fill_ratio`, the float form of a ratio the runtime never
  formats. The cycle drove the box until r42 put the player's own sharpness in
  its place.
- The sampler's tier-packed gauge snapshot (10 bits current, 10 maximum, 6 tier)
  and its `TAKEN` counter. What the row check needs is one plain question: did
  the accepted sample carry a gauge, meaning a positive maximum and points that do
  not pass it. Stage 34 asks exactly that.
- The sample's extra fields. `FIELDS` is 11 words: the controller, the animation
  row's identity tuple, the hunter index, the player and its sharpness trio. Out
  went the owner, cache and cache flags, the tree root, the whole node walk, the
  draw context and its live clip, `mInputFlags` (the riding-state candidate,
  disproved: that word is the tier), `mCmdTblId` (it named the weapon in the
  per-sample lines that no longer print), and with them `HEAD_WORDS`,
  `NODE_WORDS`, `KEY_FIELDS`, the walk field indexes, `gauge_of` and
  `ratio_of_words`. The node walk and the clip read existed to feed the per-node
  tree lines and the clip readback, deleted in r43.
- The `CUT_START_DEFAULT` / `CUT_END_DEFAULT` pair as absolute screen pixels,
  128.0 and 200.0 on the bar's 110..238 rect. core keeps the same accepted pair
  as offsets on that rect (18 and 90 of its 128 px) and scales them by the live
  rect width, so a bar laid out at another width cuts the same fraction of its
  blade. `the_accepted_cut_lands_on_the_measured_bar_at_every_ratio` pins 18 / 36 /
  54 / 90 px widths, rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 /
  100%, which is what r43's constants produce on that rect;
  `the_cut_scales_with_the_bar_rect_the_row_reports` pins the scaling at 64, 128
  and 256 px rects.

Names and wording, the same pass: `FORCED` / `FORCED_WORDS` / `take_forced_line`
are `CUT` / `CUT_WORDS` / `take_cut_line`, because the ratio is the player's, not
forced; the startup line says the mod clips the gauge to the player's sharpness
instead of "diagnostic; no drawing changes", which stopped being true at r17;
`gui::on_teardown` no longer says nothing in this build touches the tree;
`sampling::clear_controller` names the row's teardown instead of calling the
destroyed object a controller (it is the row root, whose parent is the
controller); "dragging the slider" left with the panel; and the run narratives
left `src/` and `core/src/`, so `grep -nE '\br[0-9]{1,3}\b' src core/src` is
empty. The facts those comments carried are the sections above: the 2,512 lines of
zero-width rectangles, the 376 of 383 with `left == right`, the row's 4 objects
against 3 tree nodes and its `0x727272` crash, `anchors=0xfffffff0`, the 3 objects
and 16 sequences with the pointer 192 into the allocation, the freed-heap fills at
a row's drop, and the authored-88 versus rect-128 screenshots.

README.md is now the user guide. The tester steps, the report list, the log
guide, the stage-code reference and the run-measurement note are in `TESTING.md`,
which stays outside the ZIP, and `EXPORT.json` was refreshed for it.

20 host fixtures pass, from 22: `head_field_indexes_point_at_their_names`,
`the_prototype_fill_shares_one_boundary_column`,
`the_mask_box_covers_the_bar_and_the_effect`,
`the_fill_track_scales_with_the_live_bar_width` and
`the_forced_ratios_step_the_box_from_the_rect_width` tested only the removed
track, cycle and head block, and `the_box_endpoints_are_linear_and_never_leave_the_rect`
keeps `span_box`, the maths `cut_box` drives. `src/*.rs` is 1894 lines and
`core/src/lib.rs` 547, from 1927 and 745; the audited `src/svc.S` (38 lines) is unchanged.
NRO `c3da58333cdf4c141fb39ce691cc8e3fa0560251552093e1d659cb850800131f`, ZIP
`build/modern-sharpness-gauge/candidates/r44/delivery/modern-sharpness-gauge-r44-candidate.zip`
sha256 `19af9431338202171c70b5991d455871e32d6df8b725d93d03270cfddc2f0d00`.

Where r44's runtime differs from r43, and where it cannot. The hook set, the
signatures, the gates, the map and cache growth, the link order, the mask's writes,
the glow, the log sites and `svc.S` are the same code. On the accepted bar rect
(110..238, 128 px) `cut_box` produces r43's box at every ratio: widths 18 / 36 / 54
/ 72 / 90 px and rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%,
pinned by `the_accepted_cut_lands_on_the_measured_bar_at_every_ratio`. Three
differences exist, none of them visible on a 128 px bar:

- a bar laid out at a rect width other than 128 px. r43 kept the endpoints at
  absolute screen pixels, so a bar drawn narrower cut the same pixels and so a
  larger share of a wider bar; r44 scales the offsets, so it cuts the same fraction.
  This is what the HD Project run tests.
- the stage-34 gate. r43 asked `maximum != 0` against the snapshot's 10-bit-truncated
  word, so a negative maximum read as 1023 and passed; r44 asks `maximum > 0` on the
  word as read and rejects it. No sharpness weapon reports a negative maximum, so this
  is a stricter gate on nonsense state, not a change in play.
- the gauge-present flag is now cleared at a row's teardown. A row check between
  teardowns reports stage 34 where r43 reported 21, and discovery restarts with it.

Discovery also reads less: 11 copied words where it copied 134.

Gameplay NOT RUN on r44's bytes. The accepted behaviour is on r42's, and the
cut's arithmetic is proven on the host: same endpoints, same boxes on the
measured bar, and the scaling that only shows up on a bar laid out at a
different width.

## Open items, recorded at r44 (2026-10-30)

1. Closed by r44's run: the accepted 128 / 200 px pair with the MHGU HD Project
   active. The measurement came from the r42 run whose record says HD was
   switched off for it, and r44 kept the pair as offsets on the bar's own rect,
   so the open part was whether the accepted numbers belonged to HD's layout. The
   r44 run played with HD on and reported the cut where r43 put it, with the bar
   rect `110 38 238 166` on every line, the same 128 px rect as the accepted runs.
2. Open: which thread runs the draw walker at main+0xa494a0? `insert::run` changes the
   gauge row's tree, map and cache from `forge_onUpdate`, and Forge calls that
   after the game's update entry at main+0x3da8fc, which the saved index names
   `sFestaMain::move`. The mask's own writes happen inside the walker's draw on
   whatever thread reaches the walker, and the two paths share the captured
   addresses through atomics with no lock. Searched with
   `./scripts/tool.sh research find 014124a0 --kind symbols` (the walker's own
   `FUN_014124a0`, and `main+0xa39300`'s instance entry, which takes the
   controller as its first argument), then
   `work/modern-sharpness-gauge/scripts/xref.py 0xa494a0` and `xref.py 0xa39300`
   for `bl` sites, `find_word.py 0xa494a0` for stored words equal to the load
   address, and a scan of `work/research-tooling/dti` for the same number as a
   decimal. No `bl` site, no stored word and no DTI property names either entry:
   both are reached through pointers this index does not list, so the caller's
   thread is not provable from the saved Ghidra and DTI data. What is proven from
   the runs is the behaviour, not the thread: the insertion, the mask's writes and
   the facts line all complete on a row, and the mask's enter handler is reached
   every frame while the row is drawn. r44 changes no code for this; a fix needs
   the thread identified first.

## r44 run 2026-10-30 (Eden): the accepted cut, with the HD Project on

The tester ran r44 on Eden. Installed NRO `c3da58333cdf4c141fb39ce691cc8e3fa0560251552093e1d659cb850800131f`
= the packaged payload. Log copy `work/modern-sharpness-gauge/runs/r44-eden-2026-10-30.txt` (sha256
`f2960a93370f283e00b7ae22db4727bb44d74f62b0a95c22ce902e5c152cc9e5`), 784 lines, 224 of ours.

"its ran, successfully", and all three of the checks TESTING.md asked for: the melee steps at
full, low and empty sharpness with a sharpen back, the MHGU HD Project mod on, and a quest that
crossed an area's loading screen and ended. On where the cut lands: "Same as r43".

The log agrees. 209 fill lines, ratios from 10000 down to 0, every one on bar rect `110 38 238
166` with `feed=128`, and the box at the accepted endpoints: width 18 at ratio 0 (right 128 px),
54 at 5000 bp (164 px), 90 at 10000 bp (200 px), the smallest width seen 18 and the largest 90.
Three insertions (t=117.4, 1936.5, 2596.7), each with a 5-entry map and a 9-record cache, and
three `links lost` teardowns (t=1319.0, 2266.5, 2889.0) with a new insertion after each, so three
gauge rows were built and dropped in one session. One `gauge controller observed`, one row check
at stage 21 at t=27.9 before any gauge HUD existed, four HUD-absent stretches, no insertion
failure and no stage 34. Eden logged 33 of its own `maxwell_3d` Critical asserts, with no
ForceStop, no host SIGSEGV and no guest exception.

What this settles: the accepted 128 px at 0% and 200 px at 100% work with the HD Project active,
which was an open item, and r44 is accepted on its own bytes. What it does not settle: every line's bar rect is the measured 110..238, so the scaled
offsets have only been seen where they reproduce r43's numbers, and the row check after a
teardown never printed stage 34, so the cleared flag shows up as a clean restart, not a stage.

## r44 promoted, 2026-10-30

"ready to distribute". The candidate bytes ship unchanged: NRO
`c3da58333cdf4c141fb39ce691cc8e3fa0560251552093e1d659cb850800131f`, candidate ZIP
`19af9431338202171c70b5991d455871e32d6df8b725d93d03270cfddc2f0d00`, kept in
`backups/accepted/modern-sharpness-gauge/r44-distribution-approved/` with the records as they
stood at approval. `DISTRIBUTION.json` now reads READY_TO_DISTRIBUTE with that approval, the
archive is `modern-sharpness-gauge-r44.zip` (sha256
`ee23ed17c52d84bbcc4ffdee3fac73d80099132bef84ede238162795705b0022`), and the release folder is
`build/modern-sharpness-gauge/releases/r44/`. The approval quotes the Eden run above and says
plainly what it does not cover: native Switch hardware, Ryujinx, a bar rect other than 110..238,
and the draw walker's thread. Nothing was rebuilt for promotion.

## r45, built 2026-10-30: Phase 5, the dimmed empty blade

The plan's Phase 5 asks for a dimmed, non-pulsing copy of the blade beyond the fill
boundary, same sprite, constant dark colour, no glow, and no disturbance of the
native `VertexColor` keyframes on `Txt_sharp`. The mechanism, all recorded with
sources in `work/modern-sharpness-gauge/PHASE5_RESEARCH.md`:

- `cGUIObjTexture` (DTI index: size 304, parent `cGUIObjPolygon`, vftable
  main+0x178fcc8) is allocated through its own MT allocator (`MtDti::getAllocator`
  main+0x7aa47c, `memAlloc` slot 0x1c) and constructed at the vftable's slot 0x00,
  main+0x866a4, so its vptr, lists and defaults are the game's own.
- The render fields are copied from the bar live: the transform rows at +0x10, the
  kind word at +0x30 and its pair at +0x34, the visible and drawable bits at +0x54,
  owner +0x74 and resource +0x78, the authored pair at +0xb4, the texture id and
  object at +0x100/+0x104 (via the DTI setter main+0xa4faf0), and the atlas
  rectangle at +0x10c..+0x118. The four corner colours at +0xa4..+0xb0 are written
  flat; the texture setters show colour is per corner (main+0xa4dd10 writes
  word +0xa4, main+0xa4de20 writes +0xa4 and +0xa8).
- No animation entry means no keyframe ever writes the node, which is the plan's
  requirement met by construction, and it is also the reason its rectangle has to be
  written by us: the same layout fact that made the mask's box a per-frame write.
- Placement: `root.child = mask`, `mask.sibling = dim`, the native sibling linker
  main+0xa4844c writing the sibling link and the parent, so the copy is drawn after
  the mask's subtree, outside the scissor, with one inbound chain like every other
  node in this tree.
- Geometry: `dim_shape` in core. Screen rect from the cut's right edge to the bar
  rect's right edge, atlas `u` = (box width) * 88 / rect width. On the measured bar:
  0% gives rect 128..238 with atlas u 12, 50% gives 164..238 with u 37, 100% gives
  200..238 with u 61, the strip past the tip where the sprite has no ink.
- Storage: the row's object array grows to 6 words (main+0x5456e4 indexes 6 for the
  5 nodes plus the controller) and the draw block is sized for 5 objects: 88 bytes,
  11 records, exactly the walker's 11 with the edge glyph visible and 9 in the
  looping tiers. `a_second_node_fits_the_grown_tree_exactly` pins the count, the
  array length, the record sequence and those sizes.
- Failure is additive: stages 50 (allocation), 51 (constructor and class test), 52
  (fields). A failure leaves the mask insertion exactly as the accepted r44 shipped
  it, hides the copy, and prints `dim blade not added at stage N` once per row.
- Not proven: the MT `color` word's byte order (the dark constant is byte-symmetric
  grey with alpha written high, labelled as an inference; the facts line now prints
  the bar's four corner words so a run pins the packing against the colour on
  screen), and whether the copy reads correctly under the glow's edge. The plan's
  fallback for a copy that looks wrong is recorded in the plan: keep the empty side
  clear, as r44 does, and report it.
## r45 run 2026-10-31 (Eden): the game froze about five seconds into the quest

Two quest launches on the packaged bytes (the installed NRO hashed
`6aecfb20524267678687a465aa54c4902b3b26febec27577802b03052003cca3`, the payload in the candidate
ZIP). Both went the same way: the mod started, the title-screen row check printed stage 21, the
watchdog printed `no sharpness gauge controller seen in 6002 walker calls`, then
`gauge controller observed` at t=250.36 - and the insertion reported nothing. No
`scissor mask inserted`, no `mask insertion failed at stage`, no `dim blade not added at stage`.
Those three lines are printed by `run()` on every path, success and failure, so `run()` never
returned. 1.5 seconds later, when the gauge HUD drew, the guest faulted on
`Unmapped Read32 @ 0x00000000000000` and `Unmapped Read8 @ 0x00000000005ce4` after three
`maxwell_3d` asserts, and the session was stopped by hand. The only new native calls in that path
were Phase 5's. Log: `work/modern-sharpness-gauge/runs/r45-eden-2026-10-31-frozen.txt` (sha256
`773c006244f885c8d3097378913c45ed40ee769d3c3b14d3b6e4c95ba599a326`), 6 lines from this mod, none
from the insertion.

The workspace already had the verified recipe for this class, and r45 did not use it.
`cGUIObjTexture`'s DTI has three virtual slots and vtable+0x0c is null, so the class has no
placement constructor, and its vftable slot 0x00 (main+0x866a4) is not one. The Key Quest
Markers badge, shipped and played on this class, allocates through the DTI allocator's slot 0x20
with (bytes, align, dti id), zeroes the 304-byte block, and calls the standalone constructor
main+0xa4f60c, which takes the storage in r0 and returns it
(`.agents/skills/mhgu-forge-modding/references/native-gui-extensions.md`,
`mods/key-quest-markers/src/gui.rs`). r45 called slot 0x00 on a block straight out of the
recycled pool, so every field the call reads was freed-heap fill. That is the null read in the
log, and the GUI thread died with it.

Two more faults in the same block, which that run never reached: the atlas rectangle was written
as integers where the bar's own pair reads live as floats (`tex=0 1133248512 1118830592
1134034944`, i.e. `[0, 280, 88, 304]`), and the stage-52
path returned the block to its allocator *and* called the class destructor, which frees it again.

## r46, built 2026-10-31: the copy built the way the shipped badge is built

`build_dim` follows the recipe: allocator slot 0x20 with `dti.id()`, 304 zeroed words, the
constructor at main+0xa4f60c with the pointer required back, the class test, then the fields that
recipe sets - the marker at +0x50, the bar's owner and resource at +0x74/+0x78, position at
+0x80/+0x84 as float pixels from the bar's rect corner, the bar's authored pair at +0xb4 in float
bits, the drawable and layout-dirty bits at +0x54, the texture id through main+0xa4faf0 with the
resolved object at +0x104 required, the bar's atlas rectangle as floats at +0x10c.., and the four
flat corner colours at +0xa4. Ownership follows the badge's rule: before the constructor the block
goes back to its allocator, after it only the destructor frees it. When the copy joins the
mask's sibling chain the mask gets the layout-dirty bit, the badge's way of telling the walker a
chain changed under a node. `write_dim` writes the rect as signed pixels and the atlas as floats.
No field is copied from the bar any more, and no transform row is touched.

Core, geometry, the mask, the map and cache growth, the links and the cut are untouched, so the 22
host fixtures are the same fixtures. The open questions are unchanged: the MT `color` word's byte
order, how the copy reads under the glow's edge, and the draw walker's thread.
## r46 run 2026-10-31 (Eden): the cut is normal, the copy refused itself at stage 52

No freeze, no fault: 76 lines from this mod, 68 cut lines with the accepted endpoints (`box=110 32
90 32` at 10000 bp down to `box=110 32 18 32` at 0), one insertion, one links-lost teardown at an
area change followed by the watchdog line. The insertion line says what the tester saw:

```
MSG r46 scissor mask inserted: mask=0x48081c80 map=0x47ffb570 entries=5 cache=0x48081da0 records=9 dim=0x0
MSG r46 dim blade not added at stage 52
```

So nothing was drawn behind the shorter gauge because no node existed: the additive path hid it and
the gauge looked exactly as r44. Stage 52 is the field step, and of its writes only one can fail on
a writable block: the texture resolve. Log:
`work/modern-sharpness-gauge/runs/r46-eden-2026-10-31.txt` (sha256
`4e6662865a982833eb18e9ac2702240242da2650efe50145b215cc0369669758`).

Read with sources, the setter is the gate (`work/research-tooling/ghidra-export/function_00a4faf0.c`):
it returns early when the object's resource at +0x78 is 0, and otherwise resolves the id through
main+0xb80fd0, which scans `*(obj + 0x64) + 0xe8` for `count` 16-byte entries at `+0x70`, then
zeroes the result when the found entry's +0x28 is 0. The class's base constructor (main+0xa4d0d8,
reached from main+0xa4f60c) writes 0 at +0x64, and r46 wrote +0x74/+0x78, the position, the authored
pair, the flags, the atlas rectangle and the colours, but never +0x64. A lookup through 0 finds
nothing, so +0x104 stayed 0 and stage 52 dropped the node.

## r47, built 2026-10-31: resolve the texture after the fields

Reading the setter (`work/research-tooling/ghidra-export/function_00a4faf0.c`) against the code
shows the order was wrong as well as the missing word: its first test is the object's resource at
`+0x78`, and on a freshly zeroed block it returns at once with `+0x100 = -1` and `+0x104 = 0`. r46
called it as the first thing after the constructor, so every copy it built was refused by its own
`+0x104` check. r47 writes the marker, `+0x64`, owner, resource, position, authored pair, flags,
atlas rectangle and colours first, and resolves the texture last. The insertion line gained
`dim_tex=` (the texture object the copy resolved) and `dim_flags=` (its flag word).
## r47 run 2026-10-31 (Eden): the copy is in the tree, and its drawing is the photo's likely noise

`scissor mask inserted: mask=0x48620060 map=0x485ac190 entries=6 cache=0x48620180 records=11
dim=0x44246540 dim_tex=0x519bbab0 dim_flags=0x20a0001`, 48 lines, no insertion failure, no guest
fault. The node exists and its texture resolved. `dim_flags=0x20a0001` is the class constructor's own
default: main+0xa4d0d8 ends its field initialisation with `movw r0,#1`, `movt r0,#0x20a`,
`str r0,[r4,#0x54]`, so the word the log prints is what the constructor wrote and says nothing about
whether the node draws. Nothing of the copy appeared in the sharpness gauge.

The tester's photo of that build
(`6158ff20dcb8b60d8cfdf81225e2cc141c59a624116d6acb3d13227e2228f266`) shows a noisy dark translucent
block near the gauges. Reading that block as this mod's copy is **inference**: it appears in the one
build whose copy could not sample its texture, and in none of the later builds. r47 wrote the bar's
rect corner (110, 38) into the copy's own position pair at `+0x80/+0x84`, and wrote its atlas window
with `rect_at`, which reads the four words at `+0x10c` as integers: the bar's `v1` word 1133248512 -
the float 280.0 - went into the copy as the float 1133248512.0, so its window asked for rows around
v = 1.13e9, far outside the 88x24 sheet. Sampling outside the sheet is what a block of translucent
sheet noise looks like. Log:
`work/modern-sharpness-gauge/runs/r47-eden-2026-10-31.txt` (sha256
`dc17611a502e834c7ebbcb07ea22438f053e084559a328dc6cf24fc9f176d08d`).

The fill lines, quoted verbatim (the log prints the four words at `+0x10c` as integers):

```
fill ratio_bp=10000 box=110 32 90 32 bar=110 38 238 166 eff=0 0 0 0 tex=0 1133248512 1118830592 1134034944 bar_w=88 feed=128
fill ratio_bp=4400  box=110 32 49 32 bar=110 38 238 166 eff=0 0 0 0 tex=0 1134034944 1118830592 1134821376 bar_w=88 feed=128
```

Read as floats those are `[0, 280, 88, 304]` and `[0, 304, 88, 328]` (1133248512 = 280.0,
1118830592 = 88.0, 1134034944 = 304.0, 1134821376 = 328.0, 1135607808 = 352.0). The bar's atlas
window is therefore `[0, 280, 88, 304]`, `[0, 304, 88, 328]` and `[0, 328, 88, 352]`: the `u` pair
stays 0..88 - the whole 88-column sprite - and the `v` pair steps from tier band to tier band. The
bar does not slide a window in `v` with sharpness, and the values `tex=0 156 87 440` and
`tex=0 440 87 512` quoted in the earlier version of this section are in no log: they were invented,
and the axis reading built on them was wrong.

## r48, built 2026-10-31: the copy takes the sprite rows the bar is not drawing

`dim_shape` returns the bar's own `u` pair plus the `v` range the bar is *not* drawing - the strip's
tip end down to the bar's current `v1` - as the copy's atlas window, with its screen rect from the
cut's right edge to the tip's screen position (the 100% cut box's width: 90 px on the measured bar,
screen x 200). The tip end is read live as the smallest `v1` the bar has been seen drawing, its
full-sharpness window, so a row never taken to full sharpness draws no copy instead of guessing at
the sprite. `window_at` reads and writes those four words as floats, which is how the bar's pair is
stored. The fixture `the_dim_blade_shows_the_hidden_part_of_the_strip` pins 100/50/25/0% (lefts
200/164/146/128, rights 200, windows 156..156, 156..298, 156..369, 156..512), the uncalibrated case
(no width) and a 64 px bar (119..155). Those `v` numbers are the fixture's own inputs, taken from the
invented `tex` reading; the bar's live band in the log is `[0, 280, 88, 304]`, so `tip_v` 280 against
a bar in the orange band gave the copy the 24 rows of the yellow band.

## r48 run 2026-10-31 (Eden): same state, and the layout fact that explains it

`scissor mask inserted: mask=0x443887f0 map=0x47f6b970 entries=6 cache=0x44388910 records=11
dim=0x48043500 dim_tex=0x5194bab0 dim_flags=0x20a0001`, 67 lines, 62 cut lines, no fault, no
teardown. The sharpness gauge - the upper row, the cropped red blade - showed no copy. The grey band
with the slanted tip in the tester's photo (`476014feca70a36e881189a2986b8a24cd27219653233e0fc34196316e1b0251`)
is the demon gauge's own graphics, as the tester said; my reading of it as this mod's copy was wrong,
and the section written from that reading has been replaced. Log:
`work/modern-sharpness-gauge/runs/r48-eden-2026-10-31.txt` (sha256
`6d2da8a7e0517302f20b6e76aaf907bfe3905b9fe144fe6394b13a5f02b772ce`).

The layout fact, read from the class: a `cGUIObjTexture` is laid out from its position pair at
`+0x80/+0x84` and its authored pair at `+0xb4/+0xb8`, and that layout runs when the walker visits the
node - after this plugin's callback at the mask's enter. r48 wrote the node's rectangle at the mask's
enter and never touched the position/authored pair after the build, so the quad the layout built came
from the bar's authored 88 px and from the position r48 wrote. That position was the bar's rect corner
(110, 38) read as screen pixels, written into fields that are offsets in the parent's space: the copy
hangs off the inserted mask, so its offset has to be measured from the mask, the way the bar's own
offset is. With a 110/38 offset applied in the mask's local space, the node lands far from the blade,
which is consistent with the sharpness gauge showing nothing of it. The r48 copy also samples real
rows - its window is written with `window_at`, so no 1.13e9 sampling - and the tester's photo for this
build shows the gauge clean, so nothing of it appeared there.

## r49, built 2026-10-31: move the quad in the bar's own space

`write_dim` reads the bar's live position pair, and writes the copy's as **the bar's position plus
the kept part's width**, with its authored width as **the hidden blade's width** (tip width minus kept
width), keeping the bar's y and band height, and sets the layout dirty bit so the layout applies
them. The rectangle and the atlas window are still written in the same call, and the sprite rows are
unchanged from r48. The geometry the fixtures pin (lefts 200/164/146/128, tip 200, the `v` windows
156..156 / 156..298 / 156..369 / 156..512 - the fixtures' own inputs, from the invented `tex` reading,
not log values - and the 64 px bar at 119..155) is the widths this writes; the
place it lands is the first thing the next run has to confirm.

## r50, built 2026-10-31: print the quad the copy is given

Each cut line is followed by `fill copy rect=x y x2 y2 pos=... authored=...`: the rectangle written
for the copy, its x offset in the gauge's own space and its authored width. The point is that a run
answers "where did the mod put the copy?" from the log, so the next report does not depend on
reading a photograph, and the answer is next to the ratio, the box and the bar's atlas window on the
line above it.

## r50 run 2026-10-31 (Eden): the placement is proven, the window axis is not

119 lines, no fault, no teardown: `records=11 dim=0x4800eec0 dim_tex=0x5190bab0
dim_flags=0x20a0001`, and the new quad line, paired with each cut line:

```
fill ratio_bp=8933 box=110 32 82 32 bar=110 38 238 166 eff=0 0 0 0 tex=0 1133248512 1118830592 1134034944 bar_w=88 feed=128
fill copy  rect=192 38 192 166 pos=82 authored=1
fill ratio_bp=4133 box=110 32 47 32 bar=110 38 238 166 eff=0 0 0 0 tex=0 1134034944 1118830592 1134821376 bar_w=88 feed=128
fill copy  rect=157 38 200 166 pos=47 authored=43
```

At about 41% sharpness the copy is given 157..200 (43 px) with the quad's x offset at 47 in the
gauge's own space - the placement the accepted geometry asks for, visible in the log. The window line
is the bar's band, decoded: `tex=0 1133248512 1118830592 1134034944` is [0, 280, 88, 304] and
`tex=0 1134034944 1118830592 1134821376` is [0, 304, 88, 328], so `Txt_sharp`'s rows are the tier
colour bands (yellow `0,280,88,304`, orange `0,304,88,328`, red `0,328,88,352`) and the bar's own
window is a whole band, 88 columns wide, not a blade length. r48's copy window was `[u1, tip_v, u2,
v1]` with `tip_v` the smallest `v1` the bar had been seen drawing, i.e. 280: at the orange band that
is the 24 rows of the yellow band, and at full sharpness the hidden width is 0, so the copy had rows
to sample at the mid ratios. The `156..156` window and the "zero rows" reading in the earlier version
of this section came from the invented `tex` numbers, not from the log; with the real band the
remaining question is where the node's own layout lands its quad, which is what r49 and then r52's
read-back is for.

## r51, built 2026-10-31: the copy takes the sprite's hidden columns

`dim_shape` returns the bar's live band as the copy's `v` pair and its `u1` at the cut's column
(`kept * 88 / rect width`), with the rect running from the cut's right edge to the tip's screen
position. At the measured bar: 0% → rect 128..200 from column 12, 25% → 146..200 from 24, 50% →
164..200 from 37, 100% → nothing (left = tip = 200). A 64 px bar: 119..155 from column 12. The
tip-end calibration and the row sampling by `v` are gone; the per-frame quad from r50 (position = the
bar's position + the kept width, authored width = the hidden blade's width) stays, as does the quad
log line.

## r52, built 2026-10-31: the complete badge recipe, forced values, and the read-back

Two records decide this build. The `+0x10c` window words are floats, so the log's
`1133248512 1118830592 1134034944` is `[0, 280, 88, 304]`, a whole tier band: reading them as
integers is what put r47's copy at v ≈ 1.13e9, and `dim_flags=0x20a0001` is main+0xa4d0d8's own
field write, not a drawing signal. Geometry tuning has had five candidates with none of it seen, so
r52 changes the question to "can this node draw at all, and where does the game put it".

Construction is Key Quest Markers' `create()` field by field: the DTI allocator's slot 0x20 with the
DTI id, 304 zeroed words, constructor main+0xa4f60c, then its identity/ownership words, its authored
pair, its flags, its twelve style setters in its order with its values through the two-integer ABI,
its `Texture` id through main+0xa4faf0, and its atlas rectangle through main+0xa4fb58 given a pointer
to the bar's own live four floats. Nothing of the recipe is skipped.

Forced values, written once and never again: the copy's position pair is the bar's live pair with
core's `DIM_DROP` 32.0 on y; its authored pair is the bar's (88.0 x 24.0); its four corner colours
are opaque magenta `0xFFFF00FF`. It is linked where r51 links it, as the mask's sibling. No enter
handler writes to it: `write_dim` and the `fill copy` line are deleted.

The read-back replaces them. `read_state` takes the copy's and the bar's rect at `+0xe0`, effective
rect at `+0xf0`, 16-word matrix at `+0x10..+0x4c`, position pair and authored pair, at the mask's
enter before any write, so what prints is the layout's own output for the previous frame. It queues
on change, at most once per 30 update ticks, and `forge_onUpdate` prints `copy state ...` and
`bar state ...` with the floats as floats.

What the run has to answer: a magenta full-length blade 32 units under the gauge with no pulse. The
read-back that confirms it on the measured bar: `pos=284.0 256.0`, `size=88.0 24.0`, the rect 88 by
24, the matrix's x row 1.0, and the effective rect equal to the rect. If those land and the blade is
not on screen, the node is laid out and the gap is in the draw path: back to the walker
main+0xa494a0 for why a sibling of the scissor mask is not emitted, not a new candidate.

## r52 run 2026-10-31 (Eden): the magenta blade is on screen, and it holds

The tester's photo (`work/modern-sharpness-gauge/visual/r52/r52-magenta-photo.webp`, sha256
`a6fc2f82b722d66c76642666b4e47e1b17c76841148d993f10bce31453e4e638`) shows the gauge's own yellow
blade and, under it, a full-length magenta blade. Their report: "its working well, and it stays there
when my sharpness changed (decrease and increase) also between areas". The log was not retained in
this workspace, so the read-back lines are unread here.

Four records follow from that:

- The node type, recipe and link draw. A `cGUIObjTexture` built on Key Quest Markers' recipe and
  linked as the scissor mask's sibling is emitted and drawn. Five builds of placement geometry were
  never seen; the colour and the size, not the geometry, were what made it visible.
- The `MT color` word's byte order is alpha, blue, green, red. The constant written as `0xFFFF00FF`
  came out magenta, and the bar's own `0xff5efaff` is yellow in the same order. The inference about
  the alpha byte's position is closed.
- `#202326`, the colour the tester asked for the shadow to be, is `0xFF262320` in that order.
- The prediction of the read-back is unverified here: the copy is 88 x 24 authored units and 32
  units under the bar, so its `pos=` should read the bar's pair with +32 on y and its `size=`
  88.0 / 24.0. That is the calibration r53's conversion assumes, and its own run will print it.

## r52 run 2026-10-31 (Eden): the magenta blade is on screen, and it holds

The tester's photo (`work/modern-sharpness-gauge/visual/r52/r52-magenta-photo.webp`, sha256
`a6fc2f82b722d66c76642666b4e47e1b17c76841148d993f10bce31453e4e638`) shows the gauge's own yellow
blade and, under it, a full-length magenta blade. Their report: "its working well, and it stays there
when my sharpness changed (decrease and increase) also between areas". The log was not retained in
this workspace, so the read-back lines are unread here.

That settles four things:

- A `cGUIObjTexture` built on Key Quest Markers' recipe and linked as the scissor mask's sibling is
  emitted and drawn. Five builds of placement geometry were never seen; the colour and the size, not
  the geometry, are what made it visible.
- The `MT color` word's bytes are alpha, blue, green, red: the constant written as `0xFFFF00FF` came
  out magenta, and the bar's own `0xff5efaff` is yellow in the same order. The alpha-byte inference
  is closed.
- The shadow colour the tester wants, `#202326`, is `0xFF262320` in that order.
- The copy holds its place through sharpness drops and recoveries and across areas, so the per-frame
  write is the right mechanism and the node survives the row's own rebuild the way the mask does.

## r53, built 2026-10-31: the shadow colour, and the shadow following the cut

The colour is the tester's `#202326` (`0xFF262320`), written once at the build.

The quad is written every frame at the mask's enter, after the read-back, from `dim_quad`: the
position pair is the bar's own pair moved by the width the cut keeps, and the authored pair is the
hidden length in the bar's authored units (`px * authored / rect width`) at the bar's authored
height. The `u1` is the column at the cut's edge in the band the bar is drawing. At 0% on the
measured bar that is 18 units along, 49.5 authored units wide, from column 12; at 50% 54 units,
24.75 wide, column 37; at 100% a 1 px width in the sprite's empty tail past the tip; with no gauge,
width 0. The r52 drop of 32.0 is gone: the shadow sits in the gauge's band, where the plan wants it.

What the run has to answer: the part of the blade past the cut reads as a dark grey-blue shadow in
the gauge's own band, from the cut's edge to the tip, steady, with no glow and no pulse, while the
kept part stays the game's own. The read-back that confirms it: the copy's `pos=` is the bar's pair
plus the kept width (18 at 0%, 54 at 50%), its `size=` is 49.5 / 24.0 at 0% and 24.75 / 24.0 at 50%,
and its `rect=` spans the blade's hidden part in the bar's band.

## r54, built 2026-10-31: the shadow behind the gauge

The tester's question - why not put the full blade image behind the gauge - is the layering answer,
and the code says the game does exactly that with its own row.

- main+0xa494a0 (`FUN_014124a0`) builds a flat record list by walking the tree, `(0,node)` on entry,
  `(1,node)` on exit, `(2,…)` as the terminator, and then consumes that list in order in the same
  function: kind 0 calls vtable slots 0x38 and 0x3c, kind 1 calls 0x40 and clears the layout bit,
  kind 2 returns. No sort, no z value. Paint order is walk order.
- The GUI resource carries the layer key: `Txt_sharp` Priority 0, `Txt_sharpefx` 1, `Txt_sharpEdge` 2
  (`work/modern-sharpness-gauge/hud0-sharpness.json`), and the runs walk bar → glow → edge. The
  property is `cGUIObject::Priority`, setter main+0xa47a04, getter main+0xa479fc, a word pair at
  `+0x5c/+0x60` propagated up the parent chain. The walker never reads it, and no `bl` site reaches
  either accessor, so it orders the chain when the row is built. A node we insert at runtime gets its
  layer from its link, not from that word.
- The scissor's scope is the mask's enter→exit pair: enter (main+0xa4ba94) saves the live clip into
  `mask+0xc0..0xcc`, intersects and applies; exit (main+0xa4bcdc) restores it. A node walked before
  the mask is never inside the clip.

So: `root.child = shadow`, `shadow.sibling = mask`, `mask.child = bar`, `bar.sibling = glow`,
`glow.sibling = edge`. The shadow is the walk's first node, drawn under the gauge and outside the
scissor; the bar's ink covers its kept part and the sprite's own shape shows past the cut. The
placement writes per frame are gone - `dim_quad`, the authored/position conversion and its fixture
with it - because the node keeps the bar's own quad from the build. The colour is `0xFF262320`. The
per-frame write left is the band: the four words at `+0x10c` follow the bar's live tier band, so the
dim blade follows the tier colour, one frame late.

The tree fixture now pins the shadow's enter/exit as the walk's first pair with the mask's enter
third and the mask's exit closing the scope: kinds `[0,1,0,0,1,0,1,0,1,1,2]`, 11 records, the 88-byte
block exactly.

What the run has to answer: the shadow behind the gauge, dark grey-blue, the sprite's own blade shape,
from the cut's edge to the tip, steady, no glow and no pulse, the kept part unchanged. The open visual
question is the kept side: the shadow shows through the bar art's semi-transparent pixels, so a dark
rim along the blade's outline and under its bloom is expected on this build. If that is unwanted, r53
is the fallback and needs no new code.

## r54 run 2026-10-31 (Eden): the gauge stopped clipping, and the row's chain went 8 ms after the insertion

The tester's photo (`work/modern-sharpness-gauge/visual/r54/r54-gauge-uncut-photo.webp`, sha256
`76b7960e94acda8fcbae3f7c07519d75a0628b555efa7732f1ba4330b896a420`): the blade whole, no cut, no
shadow. Log `work/modern-sharpness-gauge/runs/r54-eden-2026-10-31.txt` (sha256
`26ae7bdb1e151959a7ba00534438460b07263dd12128baef8ee0e03827e8b87f`), quoted:

```
[ 112.097788] MSG r54 scissor mask inserted: mask=0x48868af0 map=0x487db700 entries=6 cache=0x48868c10 records=11 dim=0x48872f70 dim_tex=0x517abab0 dim_flags=0x20a0001
[ 112.106043] MSG r54 inserted mask's links lost (2 times): marker=0x4d534731 mask.parent=0x488466f0 mask.child=0x48857330 bar.parent=0x0 bar.sibling=0x0 root.child=0x0 instance hook fires=48212
[ 112.106087] MSG r54 no sharpness gauge controller seen in 48212 walker calls (HUD absent?)
```

The counts in the whole log: one insertion, 17104 links-lost lines to t=398 s, 17105 HUD-absent lines
with N climbing to 576683, zero `fill` lines and zero `copy state` / `bar state` lines.

So the mask's enter ran, found `root.child = 0` and `bar.parent = 0` eight milliseconds after the
insertion, took the links-lost path, which calls `Enable(mask, 0)`, and did that every frame. The bar
was never clipped. The gauge row never matched again.

r54's only change over r52's proven shape is the insertion point: the shadow ahead of the mask on the
controller's chain (`root.child = shadow`, `shadow.sibling = mask`) so the walk emits the shadow before
the scissor's enter record. That is the position the layering research says is needed to draw behind the
gauge, and it is the position this run shows is not safe: the row's own chain teardown followed within
8 ms, and the row never came back. r44's accepted shape (mask first) and r52's (mask first, shadow as
the mask's sibling, drawn last) both inserted, tore down at area changes and re-inserted cleanly.

What the research stands, unchanged, and what it does not buy: paint order is walk order
(main+0xa494a0, quoted in full above); the GUI file's `Priority` (bar 0, glow 1, edge 2) ordered the
chain when the row was built and the walker never reads it (no `bl` site to main+0xa47a04 or
main+0xa479fc); the scissor's scope is the mask's enter→exit pair. A node we insert gets its layer from
its link, and the link that draws it under the bar is the one that breaks the row.

Next: run r53, unchanged, which is r52's tree shape with the shadow carved to the part past the cut in
`#202326`. Open question for the behind idea, if it is ever wanted: the row's teardown path (PHASE2
records it as unread) would have to be read first, because that is what r54's chain change hit.

## Rules to keep

`DISTRIBUTION.json` is authoritative; runtime changes need a new version.
Rust plus vendored forge-rs, PIC target JSON, `-nostartfiles -Wl,-z,text`. No
game ARC/GUI edits, no asset replacement: mod-owned resources only, under
`romfs/nativeNX/plugins/modern-sharpness-gauge/` via `forge::fs::romfs::mount_point()`.
No settings means no panel. Preparing this mod never installs, activates,
launches, edits saves, publishes or pushes.
