# Tests

`core/tests/behavior.rs` runs on the host (`./build.sh test`) against a fake
memory map, with no game, emulator, ARM code or Forge. It is the only test
suite this mod needs: the plugin layer is a hook plus a copy-out, and the
decisions live in the `no_std` `core` crate.

| Fixture | Question it answers |
| --- | --- |
| `the_log_samples_refill_the_bar_at_every_tier` | the tester's logged points at kireaji row 10 and maximum 150 give the expected fill and tier |
| `the_top_band_is_clipped_to_the_maximum_so_a_buff_reads_full` | maxima 400, 425 and 450 read full at the top, and a band that stops partway fills against the clipped ceiling |
| `a_zero_width_top_band_is_never_landed_in` | a `.., 450, 450` row reads full at 450 in tier 5 and never reaches tier 6 |
| `one_tier_for_every_points_is_the_plain_ratio` | a tier rule answering 0 for any points gives plain `current / maximum` |
| `a_probe_the_rule_cannot_answer_gives_no_ratio` | an unreadable probe yields no ratio, and a readable search still answers |
| `the_fill_needs_a_positive_maximum_and_clamps_its_points` | zero and negative maxima yield nothing; points above the maximum draw full and depletion draws empty |
| `every_probe_stays_within_the_maximum` | no native probe leaves `0..=maximum` |
| `gauge_identity_is_the_fixture_tuple` | animation 4 / 4 objects / first object 29 / 15 sequences matches the sharpness gauge, and `pl_gauge0` and the stamina rows do not |
| `local_player_chain_is_validated` | session cell, hunter index 0..3, `sPlayer` array slot, null session, index out of range, empty slot |
| `gauge_fields_are_signed_points` | `+10100`/`+10104` are signed points, and a null player yields nothing |
| `controller_sample_carries_the_player_and_its_gauge` | a gauge controller's sample: the local player, its hunter index and 150/160 points |
| `other_animations_cost_nothing_and_produce_no_sample` | a non-gauge controller and a null row stop at the identity check |
| `a_gauge_without_a_player_has_no_gauge_to_fill_from` | HUD present, player absent: there is no gauge to fill from |
| `the_node_count_stops_at_its_bound` | a chain past the visit limit counts up to the limit and reports the count as a floor |
| `the_walk_plan_closes_every_drawable_it_opens` | walker port main+0xa494a0 over the tree before any insertion: 5 records |
| `the_cache_grows_with_the_object_count_not_the_sequence_prefix` | cache sizing main+0xa49b28: `(objects << 4 \| 8) + sequences * 12` |
| `the_object_array_is_sized_for_the_tree_it_was_built_with` | main+0xa47d2c's node count (+1) against the row's object array |
| `the_inserted_mask_gets_its_exit_record` | walker port over a mask with the bar, glow and edge inside it: the mask is entered first and exited last |
| `a_mask_left_open_never_stops_clipping` | a mask with the glow and edge outside its subtree emits no exit record, so this shape stays out |
| `the_accepted_cut_lands_on_the_measured_bar_at_every_ratio` | box rights 128 / 146 / 164 / 182 / 200 px on the 110..238 rect |
| `the_cut_scales_with_the_bar_rect_the_row_reports` | the same endpoints as offsets on a narrower and a wider rect |
| `the_box_endpoints_are_linear_and_never_leave_the_rect` | `span_box`, the maths `cut_box` drives: linear between its endpoints, width at least 1, never past the rect |
| `the_shade_box_and_its_clip_width_follow_the_cut` | the shade spans from the cut to the blade's end, and is empty at full |
| `the_shade_clip_mask_box_is_built_from_its_own_matrix` | the clip's translation is the box's left edge, so the shade never lands over the filled part |
| `the_shade_clip_node_fits_the_grown_tree_exactly` | 6 nodes, a 7-word array and 13 records in a 104-byte area, with the copy inside the clip and after the accepted mask |

The host run covers `core` only, which is where the reads, math and identity
rules live. Hook installation, the flag audit, imports, exports, signature
windows and payload inventory are checked by `scripts/verify.py` during
`./build.sh test`. Neither run touches a game, so runtime behaviour stays
separate evidence: see `EVIDENCE.json` and `README.md`.
