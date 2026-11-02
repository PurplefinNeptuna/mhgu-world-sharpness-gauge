# Testing notes: Modern Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r63

r63 is the accepted r62 with the diagnostic lines taken out. Nothing about the drawing changed: the cut,
the shade, its worn tier frame at orange and red, its colour, transform and sampler are the accepted
r62's. The point of the run is that removing the logging does not disturb the picture.

1. One melee quest: dull the blade from full to empty and sharpen it back. The dark part grows from the
   tip side, keeps touching the cut, and shows the worn outline at orange and red, straight bottom edge
   at yellow and above.
2. Cross an area's loading screen and change weapon: the shade comes back with the new gauge.
3. The log should now carry only the lifecycle lines, one per event: `MSG r63 scissor mask inserted:
   mask=... map=... entries=7 ... records=13 dim=... clip=...`, and on the way `shade blade not added at
   stage`, `mask insertion failed at stage`, `gauge row check stopped at stage`, `gauge row root
   destroyed (N times); captured addresses cleared`, `inserted mask's links lost (N times); captured
   addresses cleared`. No fill line and no row facts line.
   `entries=7` and `records=13` are the same tree as the accepted run.
4. If the game faults, send the log.

r63 was released on static validation and has since run on Eden (2026-11-02, log only), so it has no
gameplay acceptance of its own - the picture it draws is the accepted r62's, and the byte change is
logging only. Still open: an orange crop (the accepted crop is red), the shade across a sharpening in
one crop, the dark part's missing halo as a design question, and Ryujinx and a real Switch.

By design the coloured part keeps the glow's halo, about 3 px above the blade, 2 px below it and 4 px past
the tip, and the dark part has no glow. The box rights on the bar's measured 110..238 rect are 128 / 146 /
164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%.

Tell me if the bar is missing at any point, a blank or wrongly coloured box appears over the gauge, the cut
is in the wrong place, the glow or the edge glyph is missing, or the game stops responding. Say which weapon
the quest started with and whether it ended on the results screen.

## Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r63`.

## The log lines

The per-frame instruments are gone in r63, on purpose: the `fill ratio_bp` line (the box written, the
bar's rect, `eff=`, `tex=`, `bar_w=`, `feed=`), the `shade_box=` and `shade_tex=` read-backs and the
`gauge row facts:` line located the game's vertex cache, and `EVIDENCE.json` keeps what they printed.
Their removal is the whole byte change from r62.

`scissor mask inserted` means the mask went into the gauge: `mask=` and `map=` the
nodes, `entries=` the map entries (7), `cache=` and `records=` the draw cache (13 records on a
six-node tree), `dim=` the dark copy's node and `clip=` its clip mask. `dim=0x0` with
`shade blade not added at stage 52` means the copy was refused, and the gauge is exactly the shipped
look. Report both. `mask insertion failed at stage` plus a number from 40 to 49 means it
did not go in, and the number says which step stopped it: 40 the mask object, 41
the fields it borrows from the bar, 42 the map allocation, 43 copying the map, 44
the game's cache methods, 45 the cache allocation, 46 publishing the cache, 47 the
tree links, 48 the row's counts disagreeing with its tree, in which case the gauge
draws unmodified, 49 the gauge already carries a mask from an earlier quest.
Report the number exactly as printed.

`inserted mask's links lost (N times); captured addresses cleared` means the game
took a gauge row away at an area's loading screen: the mod turns the old mask off and looks for the
new area's gauge. `gauge row root destroyed (N times)` is the same event by the row's own destructor.
Report how many areas you crossed and whether the bar behaved the same in each.

`gauge row check stopped at stage` plus a number from 21 to 34 says the gauge HUD
was not readable: 21 means no gauge HUD (the hub, a finished quest, a weapon with
no sharpness), 24 the row counts, 25 the root object's class, 26 the gauge's draw
cache, 27 the bar under it (its parent must be the root, or the inserted mask), 28
the glow beside the bar, 29 the edge beside the glow, 30 a chain that does not
close back on its parent, 31 the bar's class, 32 the bar's authored width, 33 a
missing class record, 34 no sharpness gauge to read.

The gauge HUD exists only while a weapon-owning player is in a quest: the hub, a finished quest and a
weapon without sharpness draw no gauge at all, and the mod waits there and retries, reporting only
`gauge row check stopped at stage 21`. The two lines that used to narrate that - `gauge controller
observed` and `no sharpness gauge controller seen in N walker calls` - are gone in r63. `gauge row root
destroyed (N times)` is the game's own teardown of a gauge row; `inserted mask's links lost` is the
teardown the mod can prove, because a mask left over from a finished area keeps being entered after its
links are gone, and writing into it then would write into freed memory.
