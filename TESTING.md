# Testing notes: Modern Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r52

r52 is a visibility test, not a finished look. The dulled part of the blade is drawn as a copy of
the sprite in opaque magenta - a colour the sharpness gauge never uses - at the sprite's full 88x24
size, 32 units under the sharpness gauge, written once when the gauge row is built and never moved
after that. Watch the strip below the sharpness gauge, at any sharpness level. The expected picture
is a magenta full-length blade under the gauge, steady, no glow and no pulse. Say plainly whether
you see it at all: "nothing", "a magenta blade under the gauge", "a magenta block somewhere else",
and where. If you see it, say how wide it is relative to the red blade and whether it moves.

r45 froze the game five seconds into a quest, r46 refused to add the copy, and r47, r48 and r50 all
log the copy built with its texture resolved (`dim=`, `dim_tex=`) with nothing of it visible in the
sharpness gauge. Their `dim_flags=0x20a0001` is the class constructor's own default, so it says the
copy was constructed and nothing about whether it drew. The grey band above the red cropped blade in
the r48 photo is the Dual Blade's demon gauge, a row this mod does not touch; the noisy dark block in
the r47 photo is read as this mod's copy, which is an inference.

What the log now answers on its own. The mask's enter reads the copy's own state, as the game's
layout produced it on the previous frame, and prints `copy state ...`; the same fields on the bar print
as `bar state ...`. Each line carries the rectangle the game computed (`rect=`), the rectangle it
draws inside (`eff=`), the 16-word matrix (`mat=`), the node's position pair (`pos=`) and its authored
size (`size=`), with the floats printed as numbers. On the gauge you measured, the copy's line should
read `pos=284.0 256.0`, `size=88.0 24.0`, a rectangle 88 wide and 24 tall, the matrix's first row
1.0, and `eff=` equal to `rect=`. Send the log; the lines that matter start with `MSG r52`.

The accepted cut itself is unchanged from r44, and its box maths is identical on the bar's measured
110..238 rect (128 px): rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%. What to
look at:

1. The magenta blade under the gauge: its length, its position relative to the red blade, and
   whether it is steady.
2. On the kept part nothing changed: the native colour, the pulse, the glow and the edge glyph
   are the same as r44.
3. The log carries the copy: the insertion line ends with `dim=0x...`. If it says
   `dim blade not added at stage 50/51/52`, the copy is absent and the gauge looks exactly like
   r44. Report the number.
4. Mid-quest weapon change and an area change: the copy and its state lines go with the row and
   come back on the new row.

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

Two log-wording changes, for when you read the file: the startup line reads
`loaded (clips the sharpness gauge to the player's sharpness)`, and a row check
after a gauge row's teardown reports stage 34 where it used to report 21.

## Accepted earlier, not the point of this run

The r44 run covered the melee steps at full, low and empty sharpness, the HD
Project mod on, an area change and a quest end, and reported the cut where r43
put it. A ranged weapon with no sharpness gauge, the hub, a non-English game and
the other HUD gauges are accepted on r42's bytes and unchanged in code, so they
are not the point of this run. The Phase 5 geometry is new, so the strip past the
tip at full sharpness (the 38 px where the sprite has no ink) and the copy's
scale at the mid ratios are the things to look at.

## Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
Lines that matter start with `MSG r52`.

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

`copy state` and `bar state` are the copy and the bar as the game's own layout left them: the
rectangle it computed (`rect=`), the rectangle it draws inside (`eff=`), the 16 numbers of the node's
matrix (`mat=`), the node's position pair (`pos=`) and its authored size (`size=`). They are read at
the mask's draw step, before the mod writes anything, so they are the game's answer to "where is this
node, and how big does the game think it is". The copy's line is the one that says where the magenta
blade landed; the bar's line beside it is the reference to compare it with.

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

## What earlier runs measured

The two last runs showed the gauge surviving sharpness changes, area changes and quest
changes, the 0% step drawing nothing, and the 100% step showing the whole blade. Their 25%
and 50% steps landed at 27% and 63% of the blade, because the cut's range ran 25 px past the
blade's tip. The range now ends at the tip, so the four steps are the whole measured gauge
in four linear parts.
