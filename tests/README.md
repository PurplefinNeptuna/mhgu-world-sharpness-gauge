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
| `controller_sample_carries_the_gauge_state` | a full controller sample: identity, chain, hunter index, clip, owner/cache/flags, node counts, atlas rectangle 0,280,88,304, 150/160 at tier 2, ratio 9375, and the 115-word copy round-trips |
| `the_fixture_walk_records_every_node` | the group node then three text siblings: addresses, flag words, kinds and both rectangles, with the hidden edge glyph walked as a sibling |
| `a_hidden_subtree_is_not_walked` | a visible child under a hidden node is never visited, matching the walker's own skip |
| `other_animations_cost_nothing_and_produce_no_sample` | a non-gauge controller and a null row stop at the identity check |
| `a_gauge_without_a_player_reports_the_tree_only` | HUD present, player absent: tree is recorded, ratio is unsupported |
| `a_broken_tree_stops_at_its_bounds` | a chain past the visit limit is truncated, and a dangling link is never followed |
| `the_forced_ratios_step_the_box_from_the_rect_width` | the plan's fixed ratios 0 / 2500 / 5000 / 10000 repeat every four 20 s steps, their boxes at the node's 128 px rect are 14 / 39 / 64 / 115 px (rights 124 / 149 / 174 / 225), the empty step stops left of the hilt glyph, and the authored 88 px feed is refuted because its 100% box ends at 190 px, inside the blade |

The host run covers `core` only, which is where the reads, math and identity
rules live. Hook installation, the flag audit, imports, exports, signature
windows and payload inventory are checked by `scripts/verify.py` during
`./build.sh test`. Neither run touches a game, so runtime behaviour stays
separate evidence: see `EVIDENCE.json` and `README.md`.
