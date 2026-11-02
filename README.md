# World Sharpness Gauge

The mod turns the sharpness bar into a continuous gauge, like the one in Monster Hunter: World. The bar measures the blade inside the colour
it has, so it refills at every colour change: the blade is shorter at each colour, and the bar is full
at the top of each one. Every hit cuts the blade back a little from the tip toward the hilt, and empty
sharpness leaves only the hilt. Sharpening moves the cut back toward the tip. The coloured part keeps
the game's own colour, pulse, glow and edge glyph.

The part of the blade past the cut is drawn in dark grey-blue, in the same blade shape. At orange
and red it shows the same worn notches as the coloured part. It has no glow and does not pulse.
At full sharpness there is nothing past the cut, so the gauge looks as the game draws it. Nothing
else on the HUD changes, and there is nothing to set up.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `world-sharpness-gauge-r65-candidate.zip`.
3. Copy the whole `world-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `world-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `World Sharpness Gauge r65`
should be listed.

## Usage

Start a quest with a melee weapon that has sharpness, and play normally. The bar measures the blade
inside the colour it has, so each colour starts full: hits walk the bar down to the colour below, and
a blade that is short at green shows a full bar there. Sharpening fills the bar back up in the new
colour, and a tier change moves it straight away, without easing. Sharpness +1 and +2 and the soul
skills raise the maximum; a weapon fresh from the box shows the colour that maximum reaches as full.

With a bowgun or the prowler, and in the hub, there is no sharpness gauge, and the mod changes
nothing.

## Update and removal

Close the emulator before updating or removing the mod. If you have the older Modern Sharpness
Gauge, delete its `modern-sharpness-gauge` folder first, or both mods will load together. To update, replace the
`world-sharpness-gauge` folder with the one from the new ZIP. To remove it, disable
the package or delete the folder from the emulator's mod directory. The mod writes no
save data. Forge has no unload step, so if you remove the mod while the game runs, the
changed gauge stays until you restart the game.

## Compatibility and known issues

Works with the base game. The MHGU HD Project is optional. Tested on Eden only:
Ryujinx and Switch hardware are untested.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
