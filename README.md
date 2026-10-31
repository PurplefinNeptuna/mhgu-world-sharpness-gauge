# Modern Sharpness Gauge

The mod clips the game's sharpness bar. A mask over the bar cuts it back from the right, so the
part you see starts at the hilt. The glow around the bar and the glyph at its edge keep drawing,
inside the same cut. Nothing else on the HUD changes.

This build cuts the bar at your own sharpness: full sharpness shows the whole blade, every hit
takes the cut a little further from the tip toward the hilt, in a straight line, and empty sharpness
leaves the hilt. The colours, the pulse and the glow stay the game's own on the part you see. There
is nothing to set up: the cut follows the sharpness the game reports.

The blade has a shadow behind it, in a dark grey-blue, the colour of the unlit side of the blade. It
is the same blade picture, drawn under the gauge, so what you see of it is the part the gauge's own
colour no longer covers: from the cut's edge to the tip. It has no glow and does not pulse, and it
grows and shrinks with the cut as the blade dulls and recovers. At full sharpness the gauge covers the
whole blade, so no shadow shows.

The mod writes into the game's gauge, and Forge has no plugin unload step, so the mask stays in
the gauge until the game rebuilds it. A quest is split into areas with a loading screen between
them, and each area builds its own gauge, so the mask goes into each area's gauge as it appears.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r54-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r54`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness, and play normally. Watch the gauge as the
blade dulls and as you sharpen it: the cut stays at the hilt, moves a step toward it with each
hit, and moves back toward the tip when the blade recovers.

A weapon with no sharpness gauge, a bowgun or the prowler rig, and the hub draw the bar as the
game draws it. The mod leaves the gauge alone there.

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
loading screen in particular. Report it with your log, and the drawing can be taken
out again.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
