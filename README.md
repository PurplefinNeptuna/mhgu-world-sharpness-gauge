# Modern Sharpness Gauge

r16 is a test build and it changes nothing you can see. The gauge looks the way the
game draws it. r16 reads the gauge and writes lines to the log, and the log is the
point of this build.

It follows r14, which added a clipping mask over the gauge bar. On the reported Eden
run that mask hid the bar, and the game crashed at the end of the quest. The mask is
gone in this build, and the Eden run of the r15 build that came before it drew the
gauge normally and closed the game clean.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r16-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r16`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness and play it to the results
screen. The gauge is the game's own gauge. What this build needs from you:

1. Whether the gauge bar is visible the whole time, and whether anything else on the
   HUD changed.
2. Whether the quest ends normally, with the results screen.
3. Which weapons you carried, and whether you switched weapons during the quest.

This build has no settings, so there is nothing to change in the overlay.

### Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it. The
lines that matter start with `MSG r16`.

`gauge row facts:` is one line per gauge HUD. It prints the pointers and counts the
mod reads: the gauge controller, its draw cache and the cache block the game
allocated for it, the animation row and its counts, the HUD instance the game hands
that controller to, the instance's object list with its four objects, each of the
three gauge textures' id and authored size, and `tree_nodes`, the number of nodes the
game finds under the controller. That last number is the one to watch: this row has 4
objects, the r15 run printed 3, and r14 had 4 tree nodes.

`gauge row check stopped at stage` followed by a number says which check stopped the
mod. 21 means no gauge HUD (the hub, a finished quest, a weapon with no sharpness),
24 the row counts, 25 the root object's class, 26 the gauge's draw cache, 27 the bar
under it (its parent must be the root), 28 the glow beside the bar, 29 the edge
beside the glow, 30 a chain that does not close back on the root, 31 the bar's class,
32 the bar's authored width, 33 a missing class record, 34 no sharpness gauge to read.
Report the number exactly as printed.

`sample N/320` lines carry the sharpness points, the tier, the maximum, the weapon id
and the walked nodes, and `tree N` lines print each node's child, sibling and parent
pointer. `no sharpness gauge controller seen in 6004 walker calls` means the gauge
HUD never appeared in that window. A line saying `gauge row root destroyed` means the
game freed the gauge HUD and the mod stopped using it; a session that plays quests
and ends with no such line is worth mentioning.

## Update and removal

Close the emulator before updating or removing the mod. To update, replace the
`modern-sharpness-gauge` folder with the one from the new ZIP. To remove it, disable
the package or delete the folder from the emulator's mod directory. The mod writes no
save data. Removing it while the game runs leaves the hooks in place until you
restart, because the game's plugin loader has no unload step.

## Compatibility and known issues

Works with the base game; the MHGU HD Project is optional. Native Switch
compatibility is unconfirmed.

r16 reads game memory and writes nothing to it, so no part of the HUD is supposed to
change. If the gauge bar is missing while this mod is enabled, say so: r14's mask was
the last version that touched the gauge, and it hid the bar.

Earlier builds of this mod only logged the gauge: r1 read the sharpness numbers, r3
measured the gauge tree, and r4 recorded the equipped weapon's id with every sample.
Those readings are what this build reports on.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
