# Modern Sharpness Gauge

r13 is a test build of the gauge bar. It adds a clipping mask over the game's
own sharpness gauge so the bar shows your current points against the maximum for
the weapon you are holding. The game keeps drawing the gauge, keeps its colours,
keeps its tier frames and keeps its effects; this mod only decides where the bar
stops.

The fill is a fixed cycle in this build, not a live reading: the bar is clipped
to 0%, then 25%, then 50%, then full, 20 seconds each, repeating. That is how the
clipping gets checked before the mod starts following real sharpness.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r13-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r13`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness. The gauge appears as it
always does. After about six seconds the mask takes over and the bar starts
cycling: empty, a quarter, half, full, 20 seconds at each step.

Take one screenshot at each of the four steps. What matters:

1. The bar fills from the hilt side (left) and the tip side is the part that
   disappears.
2. The glow and the edge line stay lined up with the bar, clipped at the same
   place.
3. Nothing appears on the empty side of the bar.
4. No other part of the HUD changes. The map, your health and stamina, the item
   icons and the quest text must look exactly as they do without the mod.
5. The colour of the bar stays the colour the game chose.

If the gauge is missing for a whole step, the mask was not added. If anything
else on screen is cut off, that is a real bug and worth reporting in detail.

This build has no settings, so there is nothing to change in the overlay.

### Which log to send

Send the whole log file from the emulator you played on, and the Forge log if you
have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used and describe the hunt in the order you played it.
The lines that matter start with `MSG r13`. A line saying
`scissor mask inserted` means the mask was added; a line saying
`mask insertion failed at stage` followed by a number says where it stopped.
Lines saying `clip x= ... width= ... ratio= ...` are the mask sizes the mod
applied, one for each step of the cycle, with the bar's live width at the end.
The stage number says which check stopped the mod: 21 means no gauge instance was
found, 24 the row counts, 25 the root object's class, 26 the gauge's draw cache,
27 the bar under it (its parent must be the root), 28 the glow beside the bar, 29 the
edge beside the glow, 30 a chain that does not close back on the root, 31 the bar's class, 32 the bar's live
width, 33 a missing class record, 34 no sharpness gauge to clip (the hub, a finished
quest, or a weapon with no sharpness). Report the number exactly as printed. Tree lines
print each node's child, sibling and parent pointer so a failed chain can be read
off the log.

## Update and removal

Close the emulator before updating or removing the mod. To update, replace the
`modern-sharpness-gauge` folder with the one from the new ZIP. To remove it,
disable the package or delete the folder from the emulator's mod directory. The
mod writes no save data. Removing it while the game runs leaves the mask in place
until you restart, because the game's plugin loader has no unload step.

## Compatibility and known issues

Works with the base game; the MHGU HD Project is optional. Native Switch
compatibility is unconfirmed.

The mask is added once per game launch and is not removed while the game runs.
The bar size in this build comes from the gauge's own drawing area, measured from
the game's files, so a weapon whose bar the game draws at a different size will
look short or long in this cycle; the screenshots are how that gets caught.
Nothing has been confirmed on a native console.

Earlier builds of this mod only logged the gauge: r1 read the sharpness numbers,
r3 measured the gauge tree, and r4 recorded the equipped weapon's id with every
sample. Those readings are what this build is built on.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
