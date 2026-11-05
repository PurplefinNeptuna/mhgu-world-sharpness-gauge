# World Sharpness Gauge

The mod turns the sharpness bar into a gauge like the one in Monster Hunter: World. The bar
measures the blade inside the colour it has, so it refills at every colour change. Every hit cuts the
blade back a little from the tip toward the hilt, and empty sharpness leaves only the hilt.

The part of the blade past the cut is drawn in dark grey, in the same blade shape. It has no glow
and does not pulse. At full sharpness there is nothing past the cut, so the gauge looks as the game
draws it. Nothing else on the HUD changes, and there is nothing to set up.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `world-sharpness-gauge-r68-candidate.zip`.
3. Copy the whole `world-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `world-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `World Sharpness Gauge r68`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness, and play normally. A bowgun or the Prowler has no sharpness
gauge, so the mod changes nothing.

## Update and removal

Close the emulator before updating or removing the mod. To update, replace the
`world-sharpness-gauge` folder with the one from the new ZIP. To remove it, disable the
package or delete the folder from the emulator's mod directory. The mod writes no save data. Forge
has no unload step, so if you remove the mod while the game runs, the changed gauge stays until you
restart the game.

## Compatibility and known issues

Works with the base game on Eden. On MHGURecomp it works beside KatLib and UIScale, and the cut follows a gauge
scaled by UIScale. Ryujinx and Switch hardware are untested.

It can load alongside plugins that hook the same game functions.
