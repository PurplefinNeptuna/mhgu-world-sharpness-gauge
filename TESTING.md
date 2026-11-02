# Testing notes: Modern Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r64

r64 changes one thing: the bar's length is measured inside the tier its points sit in, so the bar
refills at every colour change. The cut, the dark part, the worn tier frame at orange and red, the
colour, the transform and the sampler are accepted r62's, unchanged.

r64 has been played on Eden (2026-11-02): "from playing and look at the visuals it looks right, but my current gameplay only
satisfy no 1,2, and 5", so steps 1, 2 and 5 below are answered. Steps 3 and 4 are still open, and so is a log showing the gauge
set up again after a mid-quest loading screen: that run built one gauge row, tore it down at a quest end, and inserted nothing
after that.

The requirement is the tester's own: "make the gauge works like mh world's gauge, which the bar is
per tier, so for example, weapon with purple tier max, the bar will show the fill as between max
sharpness, and the sharpness point at white, then it became full again until it reach sharpness
point for blue, etc. ... thin band sharpness tier are okay, the need is approximation, so jumpy is
okay, no need for fancy smooth animations or lerp". A crop or a short video of each step is the
evidence that settles it.

1. No skill. Dull one weapon across at least two colours. Say whether the bar refills at each colour
   change, and whether the blade gets visibly shorter at each colour. The narrow bands are the top
   ones: a 5-point band moves the bar about 14 px per point, so the top colour empties quickly.
2. Sharpen from a low colour. When the sharpening animation ends, the bar should show the fill for
   the colour the new points land in: full at the top of that colour, partway through it otherwise.
3. Sharpness +1, or +2, on a weapon whose maximum lands partway through a colour. A weapon fresh from
   the box should read full in the colour that maximum reaches. Say which skill, which weapon, and
   what the bar showed at the start of the quest.
4. Heavy Polish active. The bar should hold at its fill while the edge glyph shows. Say whether the
   bar moves while the polish is up.
5. Cross an area's loading screen, and change weapon between quests. The gauge should set itself up
   again in the new area and for the new weapon.

Send the emulator log with each run. Say which emulator you used, which weapon you started with, and
what the bar did, in the order you played it. Tell me if the bar is missing at any point, if a blank
or wrongly coloured box appears over the gauge, if the cut is in the wrong place, if the glow or the
edge glyph is missing, if the bar never refills at a colour change, or if the game stops responding.

What is not proven yet and stays open after this run: a Sharpness skill has never appeared in a saved log and the 2026-11-02 play
did not use one, so step 3 is the first runtime check of the raised maximum; step 4, Heavy Polish, was not tried; the 2026-11-02
play ended the quest rather than crossing a loading screen mid-quest, so a second area's gauge on these bytes is unrecorded; the
accepted crop is the red tier, so orange is unseen; the order of the tier change against the draw is unread, so a tier drop may
show one frame of a full bar in the old colour; the dark part has no glow, by design; Ryujinx and a real Switch are untested.

## Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r64`.

## The log lines

The mod logs one line per lifecycle event and nothing per frame. `scissor mask inserted` means the
mask went into the gauge: `mask=` and `map=` the nodes, `entries=` the map entries (7), `cache=` and
`records=` the draw cache (13 records on a six-node tree), `dim=` the dark copy's node and `clip=`
its clip mask. `entries=7` and `records=13` are the accepted tree, the same as the accepted run's.
`dim=0x0` with `shade blade not added at stage 52` means the copy was refused, and the gauge is
exactly the shipped look. Report both.

`mask insertion failed at stage` plus a number from 40 to 49 means the mask did not go in, and the
number says which step stopped it: 40 the mask object, 41 the fields it borrows from the bar, 42 the
map allocation, 43 copying the map, 44 the game's cache methods, 45 the cache allocation, 46
publishing the cache, 47 the tree links, 48 the row's counts disagreeing with its tree, in which
case the gauge draws unmodified, 49 the gauge already carries a mask from an earlier quest. Report
the number exactly as printed.

`gauge row check stopped at stage` plus a number from 21 to 34 says the gauge HUD was not readable:
21 means no gauge HUD (the hub, a finished quest, a weapon with no sharpness), 24 the row counts, 25
the root object's class, 26 the gauge's draw cache, 27 the bar under it (its parent must be the root,
or the inserted mask), 28 the glow beside the bar, 29 the edge beside the glow, 30 a chain that does
not close back on its parent, 31 the bar's class, 32 the bar's authored width, 33 a missing class
record, 34 no sharpness gauge to read.

`inserted mask's links lost (N times); captured addresses cleared` means the game took a gauge row
away at an area's loading screen: the mod turns the old mask off and looks for the new area's gauge.
`gauge row root destroyed (N times)` is the same event by the row's own destructor. Report how many
areas you crossed and whether the bar behaved the same in each.

A bar that stays at the old `current / maximum` length is not a log event: if the fill looks like the
shipped gauge, say so, and send the log with it. The gauge HUD exists only while a weapon-owning
player is in a quest: the hub, a finished quest and a weapon without sharpness draw no gauge at all,
and the mod waits there and retries, reporting only `gauge row check stopped at stage 21`.

By design the coloured part keeps the glow's halo, about 3 px above the blade, 2 px below it and 4 px
past the tip, and the dark part has no glow. The box rights on the bar's measured 110..238 rect are
128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100% of the band.
