# Modern Sharpness Gauge

r25 draws on the game's sharpness gauge. It adds a clipping mask over the bar so
the bar shows the current sharpness ratio: at full sharpness you see the whole bar,
and as sharpness drops the bar is cut back from the right. The glow and the edge
glyph stay where the game puts them. Nothing else on the HUD changes.

The mod writes into the game's gauge. A mask built the way the mod builds it starts
with an uninitialised measurement setting and no horizontal scale, which made its box
have no width and hid the bar; the last run fixed that and the bar drew from the quest
start. What failed then was the area change: the game moved the gauge to a new area and
the old mask kept clipping, hiding the bar. r25 turns that mask off when that happens,
and the bar draws normally until the mod puts a new one in. A bar that vanishes mid-quest
is the known failure of this build; the log says whether it happened.
A quest is split into areas with a loading screen between them, and each area builds
its own gauge. The mask goes into each area's gauge as it appears.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r25-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r25`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness, and watch the gauge bar as
you hit monsters and as sharpness drops. The bar's visible width should follow
sharpness: full bar at full sharpness, a shorter bar as it drops, and an empty bar
at zero. Say whether it does.

Tell me if any of these happen:

- the bar is missing, in whole or in part, at any point in the quest;
- a blank, black or wrongly-coloured box appears over the gauge;
- the cut is in the wrong place, for example it leaves the middle of the bar and
  cuts the ends, or it cuts the bar from the left;
- the glow next to the bar, or the glyph at the bar's edge, is missing;
- the game stops responding, at the quest end in particular.

Note which weapon you carried, whether you switched weapons, and whether the quest
ended with the results screen. This build has no settings, so there is nothing to
change in the overlay.

### Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r28`.

`scissor mask inserted` means the mask went into the gauge, with the addresses it
published. `mask insertion failed at stage` plus a number from 40 to 49 means it
did not go in, and the number says which step stopped it: 40 the mask object, 41
the fields it borrows from the bar, 42 the map allocation, 43 copying the map, 44
the game's cache methods, 45 the cache allocation, 46 publishing the cache, 47 the
tree links, 48 the row's counts disagreeing with its tree, in which case the gauge
draws unmodified, 49 the gauge already carries a mask from an earlier quest.
Report the number exactly as printed.

`inserted mask's links lost (N times)` means the game took a gauge row away at
an area's loading screen: the mod turns the old mask off and looks for the new area's
gauge. The line carries the tree links as the game left them and a count of gauge
draws it still sees; a `dropped gauge row now:` line repeats those links when they
change. Report how many areas you crossed and whether the bar behaved the same in each.

`clip mask rect` lines appear every couple of seconds while the mask is in, with
the rectangle that is in the object at that moment, the measurement setting, and the
bar's rectangle it is meant to cover. The first four numbers are left, top, right,
bottom: if the left and right are equal (for example `110 32 110 64`) the clip has no
width, which is what a missing gauge looks like in the log. `anchors=` should read
with its low digits as `0` (`0xf0`), and `mat=` should start with `0x3f800000`, the
scale 1.0: a low digit of `c` in the first, or `0x0` at the start of the second, is
the missing measurement the bar needs to be visible.

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

`sample N/320` lines carry the sharpness points, the tier, the maximum, the weapon
id and the walked nodes, and `tree N` lines print each node's child, sibling and
parent pointer.

## Update and removal

Close the emulator before updating or removing the mod. To update, replace the
`modern-sharpness-gauge` folder with the one from the new ZIP. To remove it, disable
the package or delete the folder from the emulator's mod directory. Restart the game
after removing it: the mask is built inside the game's own gauge, so it stays until
the gauge is rebuilt. The mod writes no save data. Removing it while the game runs
leaves the hooks in place until you restart, because the game's plugin loader has no
unload step.

## Compatibility and known issues

Works with the base game; the MHGU HD Project is optional. Native Switch
compatibility is unconfirmed.

The mask has gone into the gauge in five runs and hid the gauge every time, with a
clip box of no width; in one of those runs the game also crashed at the quest end. The
cause is read out of the game's own code: a mask built directly keeps an
uninitialised measurement setting, and its transform has no horizontal scale, so the
box has no width. r24 sets both and lets the game do the measurement. Report a
missing bar with the log, and the drawing can be taken out again.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
