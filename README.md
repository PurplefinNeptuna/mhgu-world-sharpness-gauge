# Modern Sharpness Gauge

r3 is a diagnostic build. It reads the native sharpness gauge and writes
numbers to the log. It draws nothing and changes no game value. The gauge
itself comes in a later version, once these readings are checked.

## Requirements

- MHGU 1.4.0 for Nintendo Switch.
- [Fexty's Forge 1.5.1](https://github.com/Fexty12573/forge/releases/tag/v1.5.1).

## Installation

1. Close the emulator.
2. Extract `modern-sharpness-gauge-r3-candidate.zip`.
3. Copy the whole `modern-sharpness-gauge` folder into your emulator's MHGU mod
   directory:
   - Eden: `~/.local/share/eden/load/0100770008DD8000/`
   - Ryujinx: `mods/contents/0100770008dd8000/` inside its data directory.
4. Enable `modern-sharpness-gauge` and launch MHGU.

Open Forge's overlay and check **Loaded Plugins**: `Modern Sharpness Gauge r3`
should be listed.

## Usage

The mod works on its own. While the sharpness gauge is on screen it records up
to 320 samples per game launch, each holding:

- the animation controller that drives the gauge, and the text objects under it;
- your hunter's current sharpness points, maximum points, and the tier the game
  chose;
- the fill as basis points, where 9375 means 93.75%;
- the rectangle the game draws each text object inside, and the atlas rectangle
  that says which glyph it is.

A sample is taken when the gauge changes shape: sharpness moving a visible step,
the maximum or the tier changing, a different hunter, a recreated HUD. Otherwise
one line comes every few seconds. You do not have to force any number: play
normally and the log follows you. Each sample prints one summary line, then one line per
drawn object. The shape below is an example, not a measured r2 run:

```text
MSG r3 sample 7/320: ctrl=0x441838b0 row=0x53020238 owner=0x47f96900 cache=0x480038e0 cache_flags=0x0 root=0x488279d0 anim=4 objects=4 sequences=15 first_object=29 hunter=0 player=0x47e92570 current=294 maximum=300 tier=3 input_flags=0x0 context=0x441839c0 clip_l=0 clip_t=0 clip_r=0 clip_b=0 visited=4 visible=3 drawable=2 truncated=0 ratio_bp=9800
MSG r2 tree 7 n1: addr=0x48827a10 flags=0x2000001 kind=6 own=110 38 198 62 eff=110 38 198 62 tex=0 280 88 304
```

A line ending in `ratio=unsupported` marks a state the mod will not use: no
local player, no quest, no gauge on screen, or a maximum that is not a positive
number. The native gauge keeps drawing in all of those cases. After 320 samples
the hook stops reading game memory for the rest of that launch.

### What to play

Play 20 to 30 minutes across these quests, then send the log. MHGU gives you
one weapon per quest and a hit only takes a few sharpness points, so let the
game do the changing instead of trying to:

1. One quest with a melee weapon you use often, played through one monster or
   several, then finish or fail it.
2. In that quest, sheathe your weapon where that sharpens it, and use a whetstone
   between fights rather than in the middle of one.
3. One quest with a different weapon type, played the same way.
4. One quest where you eat before it starts, and one where you do not.
5. During a quest, open the map and the item screens. Between quests, walk
   around the hub, then start another quest: no gauge should show outside a
   quest.
6. One quest where you die or run out of time, then the next quest.
7. One online quest where you join as the second, third or fourth hunter.
8. One quest where you never draw a melee weapon.

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

r3 has not been run by its author, so its sample lines, the 320-sample cap and
the claim that nothing is drawn are build-time results, awaiting a play test.
An r1 diagnostic was played: it read the sharpness numbers correctly (300
points maximum, dropping by three per hit) but printed no tree geometry at all,
and its 60 samples spent themselves in two minutes, which r2 and r3 correct.

320 samples still cannot cover every recovery, buff and HUD state; some paths
may produce no lines in a given run, and the mod has no settings to change
that. The gauge itself is not part of this version.

If Forge reports an incompatible loader, check that MHGU is 1.4.0 and that you
are running Forge 1.5.1.

Linked components: MHGU Forge by Fexty; Original Forge by Jeffi and the Skyline
Project; forge-rs by Fexty; libnx by Hydrosphere Developers.
