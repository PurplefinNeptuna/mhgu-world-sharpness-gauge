# Testing notes: Modern Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r62

Run r62. The cut, and the shade's placement, colour and sampler, are accepted r59's, and the shade's
atlas window is the bar's own `0..88` columns with your live tier's rows. What changed is how that
window reaches the screen: the mod calls the game's own rectangle setter every frame instead of
writing the window into the node by hand. The game keeps its own copy of the window for drawing and
refreshes that copy only when the setter marks it dirty, which is why r59 and r60 kept drawing the
band that was there when the copy first drew, at every tier.

What to look at:

1. At orange: the dark part's bottom edge is worn like the coloured part's - the chips line up at the
   same columns (about 46, 58 and 70 units from the blade's left), so the worn outline runs on past
   the cut instead of stopping at it.
2. At red: more chips, and the tail of the blade rises.
3. At yellow and above: a straight bottom edge, as the game draws it.
4. As the blade dulls, the dark part grows from the tip side and keeps touching the cut's edge; as you
   sharpen, it shrinks back. Full sharpness: no dark part.
5. The kept part is untouched: the game's own colour, pulse, glow and edge glyph.
6. Across an area's loading screen and a weapon change, the shade comes back with the new gauge.
7. If the game faults, send the log.

Crops wanted: one at orange, one at red, and one across a sharpening from red to yellow.

The log line to match a crop against is the fill line's `shade_tex=`: the window the copy is drawing
with. It is expected to equal `tex=` on every fill line: `0.0 280.0 88.0 304.0` at yellow,
`0.0 304.0 88.0 328.0` at orange, `0.0 328.0 88.0 352.0` at red. The one line that can lag is the first
fill line after an insertion, which reads the band the copy was built with before that enter's setter
runs; from the next line the two numbers are the same. If `shade_tex=` follows `tex=` and
the dark part's bottom edge stays straight at orange, say so in the report: that is an answer, not a
partial pass. `shade_box=`, `bar_sampler=` and `dim_sampler=` are as in the accepted run, and
`entries=7 records=13` is the same tree. The lines that matter start with `MSG r62`.

By design the coloured part keeps the glow's halo - about 3 px above the blade, 2 px below it and 4 px
past the tip - and the dark part has no glow. Say whether the dark part's missing halo bothers you.

The accepted cut itself is unchanged from r44, and its box maths is identical on the bar's measured
110..238 rect (128 px): rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%.

1. One melee quest: full sharpness, a few hits down to a low level, empty, then
   sharpen back. The cut should land where r43 landed at the same sharpness. Take
   screenshots at full, low and empty.
2. One run with the MHGU HD Project mod on. That is the case where the bar's rect
   is not 110..238, so it is the place where the offsets scaled by the rect's width
   can disagree with r43's absolute pixels. Say where the cut sits at full and at
   empty.
3. One quest that crosses an area's loading screen and ends on the results screen.

Tell me if any of these happen:

- the bar is missing, in whole or in part, at any point in the quest;
- a blank, black or wrongly-coloured box appears over the gauge;
- the cut is in the wrong place, for example it leaves the middle of the bar and
  cuts the ends, or it cuts the bar from the left;
- the glow next to the bar, or the glyph at the bar's edge, is missing;
- the game stops responding, at the quest end in particular.

Say which weapon the quest started with (MHGU changes weapons at the box between
quests, not during one) and whether it ended on the results screen.

## Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r62`.

## The log lines

`fill ratio_bp` lines are printed when the cut moves: `0` to `10000` basis points
of your sharpness. The line carries the box the mod wrote (`box=`), the bar's own
rectangle (`bar=`), the rectangle the game draws inside (`eff=`), the part of the
sword picture the bar uses (`tex=`), the bar's authored width (`bar_w=`, 88) and
the width the box is built from (`feed=`, 128). `tex=` is the part of the sword picture the bar
uses, printed as numbers now: the gauge's window is one whole tier band, 88 columns wide, so the
line reads `tex=0.0 280.0 88.0 304.0` in the top band and steps to `304.0 328.0` and `328.0 352.0`
with the tiers. Match your screenshot to the line
with the ratio you had at that moment.

`shade_tex=` is the atlas window the copy - the dark part - is drawing with, read at the mask's draw
step before the mod writes. It is expected to equal `tex=` on every line: the copy draws the bar's own
window, so it steps with the tiers, `0.0 280.0 88.0 304.0` at yellow, `0.0 304.0 88.0 328.0` at orange
and `0.0 328.0 88.0 352.0` at red.

`scissor mask inserted` means the mask went into the gauge, with the addresses it
published, the record count, and for the copy: `dim=0x...` its node, `dim_tex=0x...` the sprite it
resolved, and `dim_flags=` its flag word - that word is the class constructor's default
(0x20a0001), so it says the copy was built, not that it draws. `dim=0x0` with `dim blade not added at stage 52` means the
copy was refused, and the gauge is exactly the shipped look. Report all three. `mask insertion failed at stage` plus a number from 40 to 49 means it
did not go in, and the number says which step stopped it: 40 the mask object, 41
the fields it borrows from the bar, 42 the map allocation, 43 copying the map, 44
the game's cache methods, 45 the cache allocation, 46 publishing the cache, 47 the
tree links, 48 the row's counts disagreeing with its tree, in which case the gauge
draws unmodified, 49 the gauge already carries a mask from an earlier quest.
Report the number exactly as printed.

`inserted mask's links lost (N times)` means the game took a gauge row away at
an area's loading screen: the mod turns the old mask off and looks for the new
area's gauge. The line carries the tree links as the game left them and a count of
gauge draws it still sees. Report how many areas you crossed and whether the bar
behaved the same in each.

`gauge row facts:` is one line per gauge HUD: the controller, its draw cache, the
animation row and both of its count pairs, the HUD instance, the instance's object
list with its entries, each gauge texture's id and authored size, and
`tree_nodes`, the number of nodes the game finds under the controller.

`gauge row check stopped at stage` plus a number from 21 to 34 says the gauge HUD
was not readable: 21 means no gauge HUD (the hub, a finished quest, a weapon with
no sharpness), 24 the row counts, 25 the root object's class, 26 the gauge's draw
cache, 27 the bar under it (its parent must be the root, or the inserted mask), 28
the glow beside the bar, 29 the edge beside the glow, 30 a chain that does not
close back on its parent, 31 the bar's class, 32 the bar's authored width, 33 a
missing class record, 34 no sharpness gauge to read.

`gauge controller observed` marks the first gauge HUD of the session, and
`no sharpness gauge controller seen in N walker calls` marks a stretch where the
game draws no gauge HUD at all: the hub, a finished quest, a weapon without
sharpness. `gauge row root destroyed (N times)` is the game's own teardown of the
gauge row; it has been seen zero times, which is why the mod relies on its own
link check.
