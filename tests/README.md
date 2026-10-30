# Tests

`core/tests/behavior.rs` runs on the host (`./build.sh test`) against a fake
memory map, with no game, emulator, ARM code or Forge. It is the only test
suite this mod needs: the plugin layer is a hook plus a copy-out, and the
decisions live in the `no_std` `core` crate.

| Fixture | Question it answers |
| --- | --- |
| `ratio_needs_a_positive_maximum_then_clamps` | partial fill, empty, full, above maximum, depleted, zero and negative maximum, extreme negative; the ratio exists only for a verified positive maximum |
| `sample_words_are_the_named_slots` | the copied sample is positional: each named slot in `FIELDS` carries the value it must, and the signed points keep their sign |
| `gauge_identity_is_the_fixture_tuple` | animation 4 / 4 objects / first object 29 / 15 sequences matches the sharpness gauge, and `pl_gauge0` and the stamina rows do not |
| `local_player_chain_is_validated` | session cell, hunter index 0..3, `sPlayer` array slot, null session, index out of range, empty slot |
| `gauge_fields_are_signed_points_and_the_tier_is_the_native_index` | `+10100`/`+10104` are signed points, `+5104` is the tier, a null player yields nothing |
| `controller_sample_carries_the_identity_and_the_player` | a gauge controller sample: identity tuple, row and controller addresses, the local player, its hunter index, and 150/160 points at tier 2 in the named slots |
| `the_fixture_walk_records_every_node` | the group node then three text siblings: addresses, flag words, kinds and both rectangles, with the hidden edge glyph walked as a sibling |
| `a_hidden_subtree_is_not_walked` | a visible child under a hidden node is never visited, matching the walker's own skip |
| `the_sharpness_textures_are_siblings_under_the_root` | the shape the row gates are written against: bar, glow and edge as siblings with the root as their parent, both ways the chain closes, and the mask's shape once it is inserted |
| `other_animations_cost_nothing_and_produce_no_sample` | a non-gauge controller and a null row stop at the identity check |
| `a_gauge_without_a_player_reports_the_identity_only` | HUD present, player absent: the identity is reported and there is no gauge to fill from |
| `a_broken_tree_stops_at_its_bounds` | a chain past the visit limit is truncated, a dangling link is never followed, and the node count the insertion gates on reports the same floor |
| `the_walk_plan_closes_every_drawable_it_opens` | walker port main+0xa494a0 over the gauge tree before any insertion: 5 records, 40 bytes, every drawable closed |
| `the_cache_grows_with_the_object_count_not_the_sequence_prefix` | cache sizing main+0xa49b28: (objects << 4 \| 8) + sequences * 12; the measured pair 3 objects and 16 sequences gives a 248-byte allocation and a 56-byte area of 7 records, and the grown 4-object row's 72-byte area is exactly the 9 records the inserted tree emits |
| `the_object_array_is_sized_for_the_tree_it_was_built_with` | main+0xa47d2c's node count (+1) against the row's object count, on the unmodified and the inserted tree |
| `the_inserted_mask_gets_its_exit_record` | walker port over the inserted tree: mask entered first and exited last, 7 records in the looping tier and 9 with the edge glyph visible |
| `a_mask_left_open_never_stops_clipping` | the r13 shape: a mask with the glow and edge outside its subtree emits no exit record, so a scissor started there never ends |
| `the_accepted_cut_lands_on_the_measured_bar_at_every_ratio` | the accepted cut on the bar's 110..238 rect: box rights 128 / 146 / 164 / 182 / 200 px at 0 / 25 / 50 / 75 / 100%, widths 18 / 36 / 54 / 90 px, the box rows covering the bar and the effect glyph's offset, and a width never below 1 or past the rect |
| `the_cut_scales_with_the_bar_rect_the_row_reports` | the same endpoints as offsets on the measured rect: 64 px, 128 px and 256 px rects cut at the same fraction of their width, and a narrow rect scales down |
| `the_box_endpoints_are_linear_and_never_leave_the_rect` | `span_box`, the maths `cut_box` drives: the right edge at the given start at 0% and at the given end at 100%, linear between, never below width 1 and never past the rect's right |

The host run covers `core` only, which is where the reads, math and identity
rules live. Hook installation, the flag audit, imports, exports, signature
windows and payload inventory are checked by `scripts/verify.py` during
`./build.sh test`. Neither run touches a game, so runtime behaviour stays
separate evidence: see `EVIDENCE.json` and `README.md`.
