# World Sharpness Gauge: fill per tier

Worked example from `mods/world-sharpness-gauge/`. Game facts it rests on are in `mhgu-understanding`.

## Fill, per tier

The ratio at `src/insert.rs`'s mask enter is the fill inside the band; the cut, the shade and the
colour code are untouched, because `cut_box` and `dim_box` take the same number.

```text
tier    = the game's own rule on the current points        (0 red .. 6 purple)
floor   = the top of the tier below, 0 in tier 0
ceiling = the top of this tier, clipped to mMaxSharpness
fill    = (current - floor) * 10_000 / (ceiling - floor)
```

The band is found by binary search on `calculateSharpnessType` (main+0x29cdfc), so the mod reads no
weapon id, level row or kireaji table, and the game's own gates apply. Points sitting exactly on a
threshold answer with the tier below, which is why `ceiling` is a point of the tier it closes; the
tier rule (`main+0xf4794`) counts thresholds strictly below the points, and all 164 kireaji rows are
monotonic, so the rule is monotonic and the search is sound. Every gate and lookup failure in the function answers tier 0 for any points, and the fill is then `current / maximum`.
A band of 5 points moves the bar about 14 px per point and then drops to the next
tier at full; 70 of the 164 rows have a zero-width band and the rule never lands in one.

The `ceiling` clip to `mMaxSharpness` is the whole buff answer, because that field is what the game
clamps points against:

| Effect | Source | What it does to the bar |
| --- | --- | --- |
| `Sharpness_Plus1`, `HonedBlade`, `BoltreaverSoul(X)` | `0xe1eec` | +1 level = +25 maximum, read through `mMaxSharpness` |
| `Sharpness_Plus2`, `HellbladeSoul(X)` | `0xe1eec` | +2 levels = +50 maximum, and takes precedence over the +1 group |
| Level cap | `0xe1eec` | Level 12, so 450 maximum. A bonus on a level 12 row adds nothing, and no melee row is level 11 |
| Whetstone and any positive `adjustSharpness` amount | 14 call sites | Current only, clamped to the maximum; sharpening has no partial steps (tester) |
| `RazorSharp`, `StonefistSoul(X)`, `SteadyHand`, `NightcloakSoul(X)`, `BluntEdge` | `0x2ad288` | The rate of loss only |
| `ChallengeSheath` | `0x2ac990` | A random gain on a condition; current only |
| `HeavyPolish` | `0x2ad288`, timer `+0x277c` | Zeroes the loss while the timer runs; its flag `+0x2844 & 0x200` switches the colour sequence to the static set. No threshold and no maximum change |

Example, great sword level id 149 with kireaji row 119 (`30 100 250 330 410 430 450`): maximum 400
ends tier 4 blue at 400, +1 gives 425 in tier 5 white, +2 gives 450 in tier 6 purple, and each reads
full at its maximum. Fixtures in `core/tests/behavior.rs` reproduce the tester's logged samples
(150, 131, 130, 100, 90, 8, 0 at maximum 150 on kireaji row 10, fills 10000, 500, 10000, 2500, 10000,
888, 0), the three buffed maxima, a `..., 450, 450` row at 450, and constant tier 0 as `current / maximum`.
No saved log was identified as using a Sharpness skill, so the +1 and +2 paths and Heavy Polish over the banded bar have no runtime check.
