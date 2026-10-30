# Testing notes: Modern Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r45

r45 adds the plan's Phase 5: the dulled part of the blade, past the cut, is drawn as a second
copy of the sprite in a flat dark colour, with no glow and no pulse. The accepted cut itself is
unchanged from r44, and its box maths is identical on the bar's measured 110..238 rect (128 px):
rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%. What to look at:

1. The part past the cut shows a dim, steady blade, the same shape, from the cut's edge to the
   tip, not a squeezed copy and not a rectangle.
2. It does not glow and does not pulse, and its colour does not breathe with the tier animation.
3. On the kept part nothing changed: the native colour, the pulse, the glow and the edge glyph
   are the same as r44.
4. Full sharpness: no dim part past the tip. Empty sharpness: the whole blade dim, hilt normal.
5. Sharpening across a tier boundary: the dim part stays put, the kept part takes the new colour.
6. The log carries the copy: the insertion line ends with `dim=0x...`. If it says
   `dim blade not added at stage 50/51/52`, the copy is absent and the gauge looks exactly like
   r44. Report the number.
7. Mid-quest weapon change and an area change: the copy goes with the row and comes back on the
   new row.

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
lines that matter start with `MSG r45`.

## The log lines

`fill ratio_bp` lines are printed when the cut moves: `0` to `10000` basis points
of your sharpness. The line carries the box the mod wrote (`box=`), the bar's own
rectangle (`bar=`), the rectangle the game draws inside (`eff=`), the part of the
sword picture the bar uses (`tex=`), the bar's authored width (`bar_w=`, 88) and
the width the box is built from (`feed=`, 128). Match your screenshot to the line
with the ratio you had at that moment.

`scissor mask inserted` means the mask went into the gauge, with the addresses it
published. `mask insertion failed at stage` plus a number from 40 to 49 means it
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
