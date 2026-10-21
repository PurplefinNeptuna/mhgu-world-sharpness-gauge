# Tests

`core/tests/behavior.rs` runs on the host (`./build.sh test`) against a fake
memory map, with no game, emulator, ARM code or Forge. It is the only test
suite this mod needs: the plugin layer is a hook plus a copy-out, and the
decisions live in the `no_std` `core` crate.

| Fixture | Question it answers |
| --- | --- |
| `ratio_needs_a_positive_maximum_then_clamps` | partial fill, empty, full, above maximum, depleted, zero and negative maximum, extreme negative; the ratio exists only for a verified positive maximum |
| `gauge_identity_is_the_fixture_tuple` | animation 4 / 4 objects / first object 29 / 15 sequences matches the sharpness gauge, and `pl_gauge0` and the stamina rows do not |
| `local_player_chain_is_validated` | session cell, hunter index 0..3, `sPlayer` array slot, null session, index out of range, empty slot |
| `gauge_fields_are_signed_points_and_the_tier_is_the_native_index` | `+10100`/`+10104` are signed points, `+5104` is the tier, a null player yields nothing |
| `controller_sample_carries_the_gauge_state` | a full controller sample: identity, owner/cache/flags, 4 nodes visited, 3 drawables, first rect, 150/160 at tier 2, ratio 9375, and the 24-word copy round-trips |
| `other_animations_cost_nothing_and_produce_no_sample` | a non-gauge controller and a null row stop at the identity check |
| `a_gauge_without_a_player_reports_the_tree_only` | HUD present, player absent: tree is recorded, ratio is unsupported |
| `a_broken_tree_stops_at_its_bounds` | a chain past the visit limit is truncated, and a dangling link is never followed |

The host run covers `core` only, which is where the reads, math and identity
rules live. Hook installation, the flag audit, imports, exports, signature
windows and payload inventory are checked by `scripts/verify.py` during
`./build.sh test`. Neither run touches a game, so runtime behaviour stays
separate evidence: see `EVIDENCE.json` and `README.md`.
