# Testing notes: World Sharpness Gauge

For the tester. The README is the user guide; this page is what to run, what to
report, and what the log lines mean.

## What to check on r68

r68 fixes a crash that r67 and earlier could cause soon after a quest starts, or later at a
loading screen: the gauge's animation state was freed while the game still used it. Ryubing 1.3.3
showed it every time; Eden rarely does. Play on Ryubing (`/mnt/storage2/Programs/Ryubing-1.3.3/run.sh`):

1. Start a quest with a blade weapon. The game must keep running past the moment the HUD appears,
   and the log must have `WSG r68 scissor mask inserted` and `bar drawn plain` with no crash after.
2. Move between zones a few times, then finish or abandon the quest and return to the village.
   Repeat with a second quest. The bar should keep cutting by tier the whole time.
3. On Eden, play one quest and confirm the gauge looks as before.

On every run, also send the `bar drawn` line. In a plain game it must say `bar drawn plain` with
`drawn` equal to `rect`. If the line is missing, the game's upload could not be read, and the cut
falls back to the game's rect.

Delete any older folder of this mod first, or both builds will load together.

The requirement is the tester's own: "make the gauge works like mh world's gauge, which the bar is
per tier, so for example, weapon with purple tier max, the bar will show the fill as between max
sharpness, and the sharpness point at white, then it became full again until it reach sharpness
point for blue, etc. ... thin band sharpness tier are okay, the need is approximation, so jumpy is
okay, no need for fancy smooth animations or lerp". A crop or a short video of each step is the
evidence that settles it.

1. Sharpness +1, or +2, on a weapon whose maximum lands partway through a colour. A weapon fresh from
   the box should read full in the colour that maximum reaches. Say which skill, which weapon, and
   what the bar showed at the start of the quest.
2. Heavy Polish active. The bar should hold at its fill while the edge glyph shows. Say whether the
   bar moves while the polish is up.

Send the emulator log with each run. Say which emulator you used, which weapon you started with, and
what the bar did, in the order you played it. Tell me if the bar is missing at any point, if a blank
or wrongly coloured box appears over the gauge, if the cut is in the wrong place, if the glow or the
edge glyph is missing, if the bar never refills at a colour change, or if the game stops responding.

A real Switch is untested. The dark part has no glow, by design.

## Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryubing: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `WSG r68`.

## The log lines

- `bar drawn plain: drawn=[...] rect=[...]`: printed once per gauge row. Nothing has moved the bar.
  The two boxes must match; a difference means the box model is wrong for this bar.
- `bar drawn rescaled: drawn=[...] rect=[...]`: another plugin moved or scaled the bar. `drawn` is
  the box the cut follows. If it shows `[0, 0, 0, 0]`, the cut uses `rect` instead.

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

`inserted mask's links lost (N times); captured addresses cleared` means the gauge row ended, which
is normal at the end of a quest: the mod turns the old mask off and looks for the next quest's gauge.
Report it if it appears in the middle of a hunt.

`fill refused; the stock bar shows while it stays refused` means the mod could not work out the fill
and left the game's own bar alone. It prints at most once per gauge row. Near a quest's end it means
nothing. Report it if the stock bar shows during a hunt.

A bar that stays at the old `current / maximum` length is not a log event: if the fill looks like the
shipped gauge, say so, and send the log with it. The gauge HUD exists only while a weapon-owning
player is in a quest: the hub, a finished quest and a weapon without sharpness draw no gauge at all,
and the mod waits there and retries, reporting only `gauge row check stopped at stage 21`.

By design the coloured part keeps the glow's halo, about 3 px above the blade, 2 px below it and 4 px
past the tip, and the dark part has no glow. The box rights on the bar's measured 110..238 rect are
128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100% of the band.
