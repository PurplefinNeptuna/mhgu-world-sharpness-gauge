# Modern Sharpness Gauge

r1 is a diagnostic build. It reads the native sharpness gauge and writes
numbers to the log. It draws nothing and changes no game value. The gauge
itself comes in a later version, once these readings are checked.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r1-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r1`
should be listed.

## Usage

The mod works on its own. While the sharpness gauge is on screen it records up
to 60 samples per game launch, each holding:

- the animation controller that drives the gauge, and the text objects under it;
- your hunter's current sharpness points, maximum points, and the tier the game
  chose;
- the fill as basis points, where 9375 means 93.75%.

Lines are written at error level, so stock Forge shows them:

```text
MSG r1 sample 7/60: ctrl=0x30c9a58 row=0x21f7c40 anim=4 objects=4 sequences=15 first_object=29 first_sequence=47 owner=0x2c9e250 cache=0x2c9e590 cache_flags=0x0 root=0x2c9e2a0 visited=4 visible=3 drawable=3 node=0x2c9e2c0 node_flags=0x20000001 rect_l=110 rect_t=38 rect_r=198 rect_b=62 player=0x5d63a40 current=150 maximum=160 tier=2 ratio_bp=9375
```

A line ending in `ratio=unsupported` marks a state the mod will not use: no
local player, no quest, no gauge on screen, or a maximum that is not a positive
number. The native gauge keeps drawing in all of those cases. After 60 samples
the hook stops reading game memory for the rest of that launch.

### What to play

Play 20 to 30 minutes across these states, then send the log:

1. One solo quest with a melee weapon you use often. Fight until sharpness drops
   through at least two tiers, then finish the quest.
2. A long whetstone to full, then a short whetstone, then keep fighting until
   the gauge turns red.
3. One quest where you eat a Sharp Dashi meal, so the gauge gains a tier during
   the quest.
4. One quest on a Light Gunner or Heavy Gunner: reload, aim, let sharpness drop.
5. Mid-quest: sheathe and draw the same weapon a few times, and switch to a
   second weapon at the item box or in camp.
6. Open and close the map and item screens, visit the tent, then start a quest.
7. One online quest where you join as the second, third, or fourth hunter.
8. End with a quest where you never draw a melee weapon.

Tell the tester anything that looked wrong on screen: a new gauge, moved text,
flicker, or a frozen HUD. None of those are expected.

### Which log to send

Send the whole log file from the emulator you played on, and the Forge log if
you have one:

- Eden: `~/.local/share/eden/log/eden_log.txt`
- Ryujinx-Nextendo: the newest file in `portable/Logs/`
- Forge log, if enabled: `forge.log` next to your game data

Say which emulator you used, and describe each quest in the order you played it.

## Update and removal

Close the emulator before updating or removing the mod. To update, replace the
`modern-sharpness-gauge` folder with the one from the new ZIP. To remove it,
disable the package or delete the folder from the emulator's mod directory. The
mod writes no save data.

## Compatibility and known issues

Works with the base game; the MHGU HD Project is optional. Native Switch
compatibility is unconfirmed.

r1 has not been run by its author, so its sample lines, the 60-sample cap and
the claim that nothing is drawn are build-time results, awaiting a play test.
The sample window is small: some recovery and buff paths may produce no lines
in a given run, and the mod has no settings to change that.

If Forge reports an incompatible loader, check that MHGU is 1.4.0 and that you
are running Forge 1.5.1.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
