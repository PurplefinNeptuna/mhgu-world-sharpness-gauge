# Modern Sharpness Gauge

The mod clips the game's sharpness bar. A mask over the bar cuts it back from the right, so the
part you see starts at the hilt. The glow around the bar and the glyph at its edge keep drawing,
inside the same cut. Nothing else on the HUD changes.

This build cuts the bar at your own sharpness: full sharpness shows the whole blade, and every
hit takes the cut a little further from the tip toward the hilt, in a straight line. Empty
sharpness leaves only the hilt. The colours and the glow stay the game's own. The mod has no
settings and no overlay panel: it cuts at the sharpness the game reports, and nothing else.

The mod writes into the game's gauge, and Forge has no plugin unload step, so the mask stays in
the gauge until the game rebuilds it. A quest is split into areas with a loading screen between
them, and each area builds its own gauge, so the mask goes into each area's gauge as it appears.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r43-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r43`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness, and play normally. Watch the gauge as the
blade dulls and as you sharpen it: the cut should track your sharpness, staying at the hilt and
moving back toward the tip as the blade recovers. Take screenshots at a few points - full, low,
empty - and say where the cut falls on the sword in each.

Tell me if any of these happen:

- the bar is missing, in whole or in part, at any point in the quest;
- a blank, black or wrongly-coloured box appears over the gauge;
- the cut is in the wrong place, for example it leaves the middle of the bar and
  cuts the ends, or it cuts the bar from the left;
- the glow next to the bar, or the glyph at the bar's edge, is missing;
- the game stops responding, at the quest end in particular.

Note which weapon you started each quest with - MHGU changes weapons at the box between quests,
not during one - and whether the quest ended with the results screen.

### Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r43`.

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
game draws no gauge HUD at all - the hub, a finished quest, a weapon without
sharpness. `gauge row root destroyed (N times)` is the game's own teardown of the
gauge row; it has been seen zero times, which is why the mod relies on its own
link check.

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

A bar that vanishes mid-quest is the known failure of this build, at an area's
loading screen in particular. The log says whether it happened: the `links lost`
line counts the gauge rows the game took away. Report it with the log, and the
drawing can be taken out again.

The two last runs showed the gauge surviving sharpness changes, area changes and quest
changes, the 0% step drawing nothing, and the 100% step showing the whole blade. Their 25%
and 50% steps landed at 27% and 63% of the blade, because the cut's range ran 25 px past the
blade's tip. The range now ends at the tip, so the four steps are the whole measured gauge
in four linear parts.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
