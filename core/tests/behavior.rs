//! Host fixtures for the gauge decisions. No game, no ARM, no Forge.
use sharpness_gauge_core::{
    basis_points, count_nodes, cache_layout, cut_box, dim_box, draw_plan, is_sharp_animation, local_player,
    read_gauge, sample_controller, scan_walk, span_box,
    CLIP_HEIGHT, CLIP_TOP,
    CONTROLLER_ROW, ENTRIES, FIELDS, FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER, Gauge, Local,
    MAX_HUNTERS, MAX_NODES, MAX_SHARPNESS, Memory, NODE_CHILD, NODE_DRAWABLE, NODE_EFFECTIVE_RECT, NODE_FLAGS,
    NODE_KIND, NODE_PARENT, NODE_RECT, NODE_SIBLING, NODE_TEXTURE_RECT, NODE_VISIBLE,
    PLAYER_ARRAY, SCAN_LIMIT, SESSION_CELL, SESSION_LOCAL_INDEX, SHARPNESS, SHARPNESS_TYPE,
    SHARP_OBJECTS, SPLAYER_CELL,
};
use std::collections::BTreeMap;

const BASE: u32 = 0x0123_4000;

#[derive(Default)]
struct Fake {
    words: BTreeMap<u32, u32>,
}
impl Fake {
    fn put(&mut self, address: u32, value: u32) {
        self.words.insert(address, value);
    }
    fn get(&self, address: u32) -> Option<u32> {
        self.words.get(&address).copied()
    }
}
impl Memory for Fake {
    fn word(&self, address: u32) -> Option<u32> {
        self.get(address)
    }
}

fn ratio(current: i32, maximum: i32) -> Option<u32> {
    basis_points(Gauge {
        current,
        maximum,
        tier: 0,
    })
}

#[test]
fn ratio_needs_a_positive_maximum_then_clamps() {
    assert_eq!(ratio(150, 160), Some(9_375));
    assert_eq!(ratio(0, 160), Some(0));
    assert_eq!(ratio(160, 160), Some(10_000));
    assert_eq!(ratio(200, 160), Some(10_000)); // above maximum draws full
    assert_eq!(ratio(-5, 160), Some(0)); // depletion floors at empty
    assert_eq!(ratio(100, 0), None); // unverified maximum
    assert_eq!(ratio(100, -1), None); // negative maximum is unsupported
    assert_eq!(ratio(i32::MIN, 3), Some(0));
}

#[test]
fn sample_words_are_the_named_slots() {
    // The copy is positional, so pin every named slot to the value it must carry.
    let mut sample = sharpness_gauge_core::Sample {
        controller: 0x6000,
        row: 0x5000,
        animation: 4,
        objects: 4,
        sequences: 15,
        first_object: 29,
        local: Local {
            player: 0x9c00,
            hunter_index: 1,
        },
        gauge: Some(Gauge {
            current: 150,
            maximum: 160,
            tier: 2,
        }),
    };
    sample.gauge = Some(Gauge {
        current: -7,
        maximum: 160,
        tier: 2,
    });
    let words = sample.words();
    assert_eq!(words.len(), FIELDS.len());
    assert_eq!(
        FIELDS,
        [
            "ctrl",
            "row",
            "anim",
            "objects",
            "sequences",
            "first_object",
            "hunter",
            "player",
            "current",
            "maximum",
            "tier"
        ]
    );
    assert_eq!(words[FIELD_CURRENT] as i32, -7); // signed points keep their sign
    assert_eq!(words[FIELD_MAXIMUM] as i32, 160);
    assert_eq!(words[FIELD_TIER], 2);
    assert_eq!((words[0], words[6], words[7]), (0x6000, 1, 0x9c00));
}

#[test]
fn gauge_identity_is_the_fixture_tuple() {
    assert!(is_sharp_animation(4, 4, 29, 15));
    // pl_gauge0 (id 7, six objects from row 33) and the stamina rows are not it.
    assert!(!is_sharp_animation(7, 6, 33, 8));
    assert!(!is_sharp_animation(3, 3, 24, 6));
    assert!(!is_sharp_animation(4, 4, 33, 15));
    assert!(!is_sharp_animation(4, 4, 29, 16));
}

#[test]
fn local_player_chain_is_validated() {
    let mut fake = Fake::default();
    fake.put(BASE + SESSION_CELL, 0x3000);
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 2);
    fake.put(BASE + SPLAYER_CELL, 0x4000);
    fake.put(0x4000 + PLAYER_ARRAY + 2 * 4, 0x9c00);
    assert_eq!(
        local_player(&fake, BASE),
        Some(Local {
            player: 0x9c00,
            hunter_index: 2
        })
    );

    // Hunter index outside 0..3 rejects the whole chain.
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 7);
    assert_eq!(local_player(&fake, BASE), None);

    // A null session cell (no quest state) is unsupported, not a crash.
    fake.put(BASE + SESSION_CELL, 0);
    assert_eq!(local_player(&fake, BASE), None);

    // An empty slot in the player array is unsupported.
    fake.put(BASE + SESSION_CELL, 0x3000);
    fake.put(0x3000 + SESSION_LOCAL_INDEX, MAX_HUNTERS - 1);
    assert_eq!(local_player(&fake, BASE), None);
}

#[test]
fn gauge_fields_are_signed_points_and_the_tier_is_the_native_index() {
    let mut fake = Fake::default();
    fake.put(0x9c00 + SHARPNESS, 150u32);
    fake.put(0x9c00 + MAX_SHARPNESS, 160u32);
    fake.put(0x9c00 + SHARPNESS_TYPE, 2);
    assert_eq!(
        read_gauge(&fake, 0x9c00),
        Some(Gauge {
            current: 150,
            maximum: 160,
            tier: 2
        })
    );
    fake.put(0x9c00 + SHARPNESS, (-7i32) as u32);
    assert_eq!(
        read_gauge(&fake, 0x9c00).map(|g| (g.current, basis_points(g))),
        Some((-7, Some(0)))
    );
    assert_eq!(read_gauge(&fake, 0), None);
}

/// Build the gauge controller fixture: animation row (id 4, packed counts 4
/// objects and 15 sequences, first object 29), one group node with three text
/// siblings, and a local player with 150/160 points at tier 2.
fn gauge_fixture() -> Fake {
    let mut fake = Fake::default();
    let (row, controller, root, sharp, efx, edge, player) =
        (0x5000u32, 0x6000u32, 0x7000u32, 0x7100u32, 0x7200u32, 0x7300u32, 0x9c00u32);
    fake.put(BASE + SESSION_CELL, 0x3000);
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 0);
    fake.put(BASE + SPLAYER_CELL, 0x4000);
    fake.put(0x4000 + PLAYER_ARRAY, player);
    fake.put(player + SHARPNESS, 150);
    fake.put(player + MAX_SHARPNESS, 160);
    fake.put(player + SHARPNESS_TYPE, 2);
    fake.put(controller + CONTROLLER_ROW, row);
    fake.put(controller + NODE_CHILD, root);
    fake.put(row, 4); // animation id
    fake.put(row + 4, 4 | (15 << 16)); // objects | sequences << 16
    fake.put(row + 0x0c, 29); // first object row
    // The gauge subtree: a visible group node, then the sharp glyph, the
    // effect glyph, and the edge glyph. The edge glyph is drawable but its
    // visibility bit is clear, which is what the looping tiers do.
    fake.put(root + NODE_FLAGS, NODE_VISIBLE);
    fake.put(root + NODE_KIND, 3);
    fake.put(root + NODE_CHILD, sharp);
    fake.put(root + NODE_RECT, 80);
    fake.put(root + NODE_RECT + 4, 20);
    fake.put(root + NODE_RECT + 8, 210);
    fake.put(root + NODE_RECT + 12, 70);
    fake.put(sharp + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(sharp + NODE_KIND, 6);
    fake.put(sharp + NODE_SIBLING, efx);
    fake.put(sharp + NODE_RECT, 110);
    fake.put(sharp + NODE_RECT + 4, 38);
    fake.put(sharp + NODE_RECT + 8, 198);
    fake.put(sharp + NODE_RECT + 12, 62);
    fake.put(sharp + NODE_EFFECTIVE_RECT, 110);
    fake.put(sharp + NODE_EFFECTIVE_RECT + 4, 38);
    fake.put(sharp + NODE_EFFECTIVE_RECT + 8, 198);
    fake.put(sharp + NODE_EFFECTIVE_RECT + 12, 62);
    fake.put(sharp + NODE_TEXTURE_RECT, 0);
    fake.put(sharp + NODE_TEXTURE_RECT + 4, 280);
    fake.put(sharp + NODE_TEXTURE_RECT + 8, 88);
    fake.put(sharp + NODE_TEXTURE_RECT + 12, 304);
    fake.put(efx + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(efx + NODE_KIND, 6);
    fake.put(efx + NODE_SIBLING, edge);
    fake.put(efx + NODE_RECT, 96);
    fake.put(efx + NODE_RECT + 4, 40);
    fake.put(efx + NODE_RECT + 8, 204);
    fake.put(efx + NODE_RECT + 12, 60);
    fake.put(edge + NODE_FLAGS, NODE_DRAWABLE); // native: hidden in looping tiers
    fake.put(edge + NODE_KIND, 6);
    fake.put(edge + NODE_CHILD, 0x7400); // below a hidden node, never drawn
    fake.put(0x7400 + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake
}

#[test]
fn controller_sample_carries_the_identity_and_the_player() {
    let fake = gauge_fixture();
    let sample = sample_controller(&fake, 0x6000, BASE).expect("gauge controller");
    assert_eq!((sample.animation, sample.objects, sample.sequences), (4, 4, 15));
    assert_eq!(sample.first_object, 29);
    assert_eq!((sample.controller, sample.row), (0x6000, 0x5000));
    assert_eq!(
        sample.local,
        Local {
            player: 0x9c00,
            hunter_index: 0
        }
    );
    assert_eq!(
        sample.gauge,
        Some(Gauge {
            current: 150,
            maximum: 160,
            tier: 2
        })
    );
    let words = sample.words();
    assert_eq!(words[FIELD_CURRENT] as i32, 150);
    assert_eq!(words[FIELD_MAXIMUM] as i32, 160);
    assert_eq!(words[FIELD_TIER], 2);
    assert_eq!(words[0], 0x6000);
    assert_eq!(words[7], 0x9c00);
}

#[test]
fn the_fixture_walk_records_every_node() {
    let walk = scan_walk(&gauge_fixture(), 0x7000);
    assert_eq!((walk.visited, walk.visible, walk.drawable), (4, 3, 3));
    assert!(!walk.truncated);
    let group = walk.nodes[0];
    assert_eq!((group.address, group.flags, group.kind), (0x7000, NODE_VISIBLE, 3));
    assert_eq!(group.rect, [80, 20, 210, 70]);
    let sharp = walk.nodes[1];
    assert_eq!(
        (sharp.address, sharp.flags, sharp.kind),
        (0x7100, NODE_VISIBLE | NODE_DRAWABLE, 6)
    );
    assert_eq!(sharp.effective, [110, 38, 198, 62]);
    assert_eq!(sharp.texture, [0, 280, 88, 304]);
    // The hidden edge glyph is still walked as a sibling and keeps its own
    // child out of the walk.
    assert_eq!((walk.nodes[3].address, walk.nodes[3].flags), (0x7300, NODE_DRAWABLE));
    assert_eq!(walk.nodes[4].address, 0);
}

#[test]
fn a_hidden_subtree_is_not_walked() {
    // The walker does not descend when the visibility bit is clear, so a
    // visible child below the hidden edge glyph is never visited.
    let fake = gauge_fixture();
    assert_eq!(scan_walk(&fake, 0x7000).visited, 4);
}

#[test]
fn the_sharpness_textures_are_siblings_under_the_root() {
    // Shape the gates in gui.rs are written against: the bar, the glow and the
    // edge are siblings whose parent is the walk root, and the walk closes when
    // a sibling comes back to that root.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, edge) = (0x7000u32, 0x7100u32, 0x7200u32, 0x7300u32);
    fake.put(sharp + NODE_CHILD, 0);
    fake.put(sharp + NODE_PARENT, root);
    fake.put(efx + NODE_SIBLING, edge);
    fake.put(efx + NODE_PARENT, root);
    fake.put(edge + NODE_SIBLING, root);
    fake.put(edge + NODE_PARENT, root);
    let walk = scan_walk(&fake, sharp);
    assert_eq!(walk.nodes[0].links, [0, efx, root]);
    assert_eq!(walk.nodes[1].links, [0, edge, root]);
    assert_eq!(walk.nodes[2].links, [0x7400, root, root]);
    // The mask enters as the root's first child and takes the bar under it, so
    // the root's child slot is no longer the bar and the bar's parent is the mask.
    let mask = 0x7500u32;
    fake.put(mask + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, efx);
    fake.put(mask + NODE_PARENT, root);
    fake.put(root + NODE_CHILD, mask);
    fake.put(sharp + NODE_PARENT, mask);
    let walked = scan_walk(&fake, mask);
    assert_eq!(walked.nodes[0].address, mask);
    assert_eq!(walked.nodes[0].links, [sharp, efx, root]);
    assert_eq!(walked.nodes[1].links, [0, efx, mask]);

    // The chain also closes with the edge glyph's sibling at 0, popping through
    // its parent, which is the root. Both closings end the walk.
    fake.put(edge + NODE_SIBLING, 0);
    let closed = scan_walk(&fake, sharp);
    assert_eq!(closed.nodes[2].links, [0x7400, 0, root]);
}

#[test]
fn other_animations_cost_nothing_and_produce_no_sample() {
    let mut fake = gauge_fixture();
    // A non-gauge controller (pl_gauge0: id 7, six objects from row 33).
    fake.put(0x6100 + CONTROLLER_ROW, 0x5100);
    fake.put(0x5100, 7);
    fake.put(0x5100 + 4, 6 | (8 << 16));
    fake.put(0x5100 + 0x0c, 33);
    assert_eq!(sample_controller(&fake, 0x6100, BASE), None);
    // A null row pointer is unsupported state, not a fault.
    fake.put(0x6200 + CONTROLLER_ROW, 0);
    assert_eq!(sample_controller(&fake, 0x6200, BASE), None);
}

#[test]
fn a_gauge_without_a_player_reports_the_identity_only() {
    let mut fake = gauge_fixture();
    // Player absent (for example a non-quest state): the identity is still the
    // gauge row, and there is no gauge to fill from.
    fake.put(BASE + SESSION_CELL, 0);
    let sample = sample_controller(&fake, 0x6000, BASE).expect("gauge controller");
    assert_eq!(sample.local, Local::default());
    assert_eq!(sample.gauge, None);
    assert_eq!((sample.animation, sample.objects), (4, 4));
    let words = sample.words();
    assert_eq!((words[FIELD_CURRENT], words[FIELD_MAXIMUM]), (0, 0));
}

#[test]
fn a_broken_tree_stops_at_its_bounds() {
    // A chain longer than the visit limit marks the scan truncated, and only
    // the leading MAX_NODES nodes are copied out.
    let mut fake = Fake::default();
    for index in 0..(SCAN_LIMIT as u32 + 16) {
        let node = 0xa000 + index * 0x10;
        fake.put(node + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
        fake.put(node + NODE_CHILD, 0);
        fake.put(node + NODE_SIBLING, node + 0x10);
    }
    let scan = scan_walk(&fake, 0xa000);
    assert_eq!(scan.visited, SCAN_LIMIT);
    assert!(scan.truncated);
    assert_eq!(
        scan.nodes[MAX_NODES - 1].address,
        0xa000 + (MAX_NODES as u32 - 1) * 0x10
    );
    // A dangling link is recorded as truncation, never followed.
    let mut broken = Fake::default();
    broken.put(0xb000 + NODE_FLAGS, NODE_VISIBLE);
    broken.put(0xb000 + NODE_CHILD, 0xcccc_cccc);
    let scan = scan_walk(&broken, 0xb000);
    assert_eq!(scan.visited, 1);
    assert!(scan.truncated);
    // The node count the insertion gates on shares those bounds: the same chain,
    // reached through a controller, counts up to the limit and reports the floor.
    fake.put(0x9000 + NODE_CHILD, 0xa000);
    let (count, cut) = count_nodes(&fake, 0x9000);
    assert_eq!((count, cut), (SCAN_LIMIT, true));
}

/// The gauge tree as the runs show it, with every link the walker follows: the
/// bar, glow and edge are siblings whose parent is the walk root, and the edge
/// closes the chain with a zero sibling.
fn gauge_tree(fake: &mut Fake, edge_visible: bool) -> (u32, u32, u32, u32) {
    let (root, sharp, efx, edge) = (0x7000u32, 0x7100u32, 0x7200u32, 0x7300u32);
    fake.put(sharp + NODE_PARENT, root);
    fake.put(efx + NODE_PARENT, root);
    fake.put(edge + NODE_PARENT, root);
    fake.put(edge + NODE_SIBLING, 0);
    // The edge glyph has nothing below it, and in the looping tiers its
    // visibility bit is clear.
    fake.put(edge + NODE_CHILD, 0);
    if edge_visible {
        fake.put(edge + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    }
    (root, sharp, efx, edge)
}

#[test]
fn the_walk_plan_closes_every_drawable_it_opens() {
    // The port of main+0xa494a0 over the tree before any insertion: enter and
    // exit for the bar and the glow, nothing for the hidden edge glyph, then the
    // terminator. 5 records is 40 bytes, inside the 56-byte record area main+0xa49b28
    // sizes for this row (2 records spare).
    let mut fake = gauge_fixture();
    let (root, ..) = gauge_tree(&mut fake, false);
    let plan = draw_plan(&fake, root);
    assert_eq!(plan.kinds[..plan.count as usize], [0, 1, 0, 1, 2]);
    assert!(!plan.overflow);
    assert_eq!(plan.count * 8, 40);
}

#[test]
fn the_cache_grows_with_the_object_count_not_the_sequence_prefix() {
    // main+0xa49b28 sizes the row's draw block as (objects << 4 | 8) + sequences * 12,
    // allocates it through the root's vtable slot 0x58 into root+0xa4, and publishes
    // root+0x80 = alloc + sequences * 12. This row's sizing pair at row+8 is 3 objects
    // and 16 sequences, with the published pointer 192 = 16 * 12 into the allocation:
    // a 56-byte record area of 7 records, and a 248-byte allocation. The inserted tree's
    // 9 records are 72 bytes, 16 past that area, which is why the insertion grows the
    // block for 4 objects: 72 bytes, exactly the 9.
    assert_eq!(cache_layout(3, 16), (248, 192, 56));
    assert_eq!(cache_layout(4, 16), (264, 192, 72));
    assert_eq!(cache_layout(0, 16), (192, 192, 0));
    assert_eq!(cache_layout(3, 0), (56, 0, 56));
    assert_eq!(56 / 8, 7);
    assert_eq!(72 / 8, 9);
    // The tree the insertion produces, measured by the walker port: 9 records, exactly the
    // grown area, and 16 bytes past the row's own 56.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, edge) = gauge_tree(&mut fake, true);
    let mask = 0x7500u32;
    fake.put(mask + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, 0);
    fake.put(root + NODE_CHILD, mask);
    for node in [sharp, efx, edge] {
        fake.put(node + NODE_PARENT, mask);
    }
    let plan = draw_plan(&fake, root);
    assert_eq!(plan.count, 9);
    assert_eq!(count_nodes(&fake, root), (4, false));
    assert_eq!(plan.count * 8, cache_layout(4, 16).2);
    assert_eq!(plan.count * 8 - cache_layout(3, 16).2, 16);
}

#[test]
fn the_object_array_is_sized_for_the_tree_it_was_built_with() {
    // main+0xa47d2c counts every node in the child and sibling chains under
    // controller+0x68 and tests no visibility bit; main+0xa38ffc returns that count plus
    // one; main+0x5456e4 uses the sum as the length of the instance's array at +0xd8 and
    // calls vtable slot 0x50 on every entry. This row's 4 objects are the root plus its
    // 3 textures, so the tree it was built with is exactly the size that pass expects.
    // An inserted fourth node makes that pass walk 5 entries through a 4-entry array.
    // Pinned so an insertion is never shipped without the map.
    let mut fake = gauge_fixture();
    let (root, ..) = gauge_tree(&mut fake, false);
    let (plain, plain_cut) = count_nodes(&fake, root);
    assert_eq!((plain, plain_cut), (3, false));
    assert_eq!(plain + 1, SHARP_OBJECTS, "the row is built for the tree it has");
    let mask = 0x7500u32;
    fake.put(mask + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(mask + NODE_CHILD, 0x7100);
    fake.put(mask + NODE_SIBLING, 0);
    fake.put(root + NODE_CHILD, mask);
    let (grown, grown_cut) = count_nodes(&fake, root);
    assert_eq!((grown, grown_cut), (4, false));
    assert!(
        grown + 1 > SHARP_OBJECTS,
        "5 indexed entries against a 4-object row"
    );
}

#[test]
fn the_inserted_mask_gets_its_exit_record() {
    // The inserted shape: mask as the root's first child, bar under it, glow and
    // edge as the bar's siblings inside the mask's subtree, and a zero sibling on
    // the edge and on the mask. The walk pops from the edge back to the mask, so the
    // mask's own exit record is emitted, which is what ends the scissor.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, edge) = gauge_tree(&mut fake, false);
    let mask = 0x7500u32;
    fake.put(mask + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, 0);
    fake.put(root + NODE_CHILD, mask);
    fake.put(mask + NODE_PARENT, root);
    fake.put(sharp + NODE_PARENT, mask);
    fake.put(efx + NODE_PARENT, mask);
    fake.put(edge + NODE_PARENT, mask);
    // Normal looping state: the edge glyph is hidden, so it emits nothing. The mask
    // is entered first and exited last, and that exit is the end of the scissor.
    let plan = draw_plan(&fake, root);
    assert_eq!(plan.kinds[..plan.count as usize], [0, 0, 1, 0, 1, 1, 2]);
    assert!(!plan.overflow);
    assert_eq!(plan.count * 8, 56);
    // Static tier: the edge glyph is visible, so it enters and its exit comes out
    // inline (byte 0x57 bit 1 is word bit 25, the drawable bit). 9 records is 72
    // bytes, the grown area, which is why grow_cache rebuilds the block for 4 objects.
    fake.put(edge + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    let full = draw_plan(&fake, root);
    assert_eq!(full.kinds[..full.count as usize], [0, 0, 1, 0, 1, 0, 1, 1, 2]);
    assert_eq!(full.count * 8, 72);
    assert!(!full.overflow && (full.count as usize) <= ENTRIES);
}

#[test]
fn a_mask_left_open_never_stops_clipping() {
    // The shape where the mask takes the bar as its child but keeps the glow and the
    // edge as siblings whose parent is the root, so the pop from the edge jumps to the
    // root and ends the walk. The mask gets an enter record and no exit record, which
    // is a scissor that never ends. Pinned so this shape cannot come back.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, _edge) = gauge_tree(&mut fake, false);
    let mask = 0x7500u32;
    fake.put(mask + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, efx);
    fake.put(mask + NODE_PARENT, root);
    fake.put(root + NODE_CHILD, mask);
    fake.put(sharp + NODE_PARENT, mask);
    let plan = draw_plan(&fake, root);
    let kinds = &plan.kinds[..plan.count as usize];
    // The mask is entered first and the walk ends at the root without ever
    // returning to it, so no exit record for the mask: a scissor that never ends.
    assert_eq!(kinds, [0, 0, 1, 0, 1, 2]);
    assert_eq!(kinds[0], 0);
    assert_eq!(&kinds[1..5], [0, 1, 0, 1]);
    assert_eq!(plan.count * 8, 48);
}

#[test]
fn the_accepted_cut_lands_on_the_measured_bar_at_every_ratio() {
    // The bar's rect as the gauge row reports it: 110..238, 128 px. The accepted
    // endpoints are 128 px at 0% and 200 px at 100% on that rect, so the cut's
    // offsets are 18 and 90 px from its left, scaled by its live width.
    let bar = [110, 38, 238, 166];
    let rights: Vec<i32> = [0u32, 2_500, 5_000, 7_500, 10_000]
        .iter()
        .map(|r| cut_box(bar, *r))
        .map(|box_| box_[0] + box_[2])
        .collect();
    assert_eq!(rights, vec![128, 146, 164, 182, 200]);
    let widths: Vec<i32> = [0u32, 2_500, 5_000, 10_000]
        .iter()
        .map(|r| cut_box(bar, *r)[2])
        .collect();
    assert_eq!(widths, vec![18, 36, 54, 90]);
    // The rows cover the bar top to bottom and the effect glyph's offset rows.
    for ratio_bp in [0u32, 5_000, 10_000] {
        let box_ = cut_box(bar, ratio_bp);
        assert_eq!((box_[1], box_[3]), (bar[1] + CLIP_TOP, CLIP_HEIGHT));
        assert!(box_[2] >= 1 && box_[2] <= bar[2] - bar[0]);
    }
    assert_eq!((CLIP_TOP, CLIP_HEIGHT), (-6, 32));
}

#[test]
fn the_cut_scales_with_the_bar_rect_the_row_reports() {
    // The endpoints are offsets on the measured 128 px rect, so a bar laid out at
    // half and at double that width cuts the same fraction of its blade: 9/45 px
    // and 36/180 px from the left, landing at the same fractions.
    let half = [110, 38, 174, 166]; // 64 px
    let double = [110, 38, 366, 166]; // 256 px
    assert_eq!(cut_box(half, 0)[2], 9);
    assert_eq!(cut_box(half, 10_000)[2], 45);
    assert_eq!(cut_box(double, 0)[2], 36);
    assert_eq!(cut_box(double, 10_000)[2], 180);
    for (rect, want) in [(half, 45.0 / 64.0), (double, 180.0 / 256.0), ([110, 38, 238, 166], 90.0 / 128.0)]
    {
        let got = cut_box(rect, 10_000)[2] as f64 / (rect[2] - rect[0]) as f64;
        assert!((got - want).abs() < 0.02, "{rect:?} cuts at {got}");
    }
    // A narrow bar scales the same offsets down and the box never exceeds its own
    // width: 2 px at 0% and 14 px at 100% on a 20 px rect.
    let narrow = [110, 38, 130, 166];
    assert_eq!((cut_box(narrow, 0)[2], cut_box(narrow, 10_000)[2]), (2, 14));
}

#[test]
fn the_box_endpoints_are_linear_and_never_leave_the_rect() {
    // span_box is what cut_box drives: the right edge sits at the start offset at
    // 0% and at the end offset at 100%, linear between, never below width 1 and
    // never past the rect's right.
    let rect = [110, 38, 238, 166];
    let (start, end) = (18, 90);
    let widths: Vec<i32> = [0u32, 2_500, 5_000, 7_500, 10_000]
        .iter()
        .map(|r| span_box(rect, *r, start, end)[2])
        .collect();
    assert_eq!(widths, vec![18, 36, 54, 72, 90]);
    let steps: Vec<i32> = widths.windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(steps, vec![18, 18, 18, 18]);
    let shifted: Vec<i32> = [0u32, 5_000, 10_000]
        .iter()
        .map(|r| span_box(rect, *r, start + 20, end + 20)[2])
        .collect();
    assert_eq!(shifted, vec![38, 74, 110]);
    assert!(span_box(rect, 0, 1, 238)[2] >= 1);
    assert_eq!(span_box(rect, 10_000, 1, 400)[2], 128, "never past the rect");
}

#[test]
fn the_shade_box_and_its_clip_width_follow_the_cut() {
    // The bar's measured rect (110..238, 128 px) with the accepted endpoints (18 px at 0%, 90 px
    // at 100%): the shadow starts at the cut's right edge and its right edge is the full-sharpness
    // right edge, so at empty sharpness it covers 128..200 px and at full sharpness its width is 0.
    let rect = [110, 38, 238, 166];
    let ratios = [0u32, 2500, 5000, 7500, 10_000];
    let lefts: Vec<i32> = ratios.iter().map(|bp| dim_box(rect, *bp)[0]).collect();
    let rights: Vec<i32> = ratios.iter().map(|bp| dim_box(rect, *bp)[2]).collect();
    assert_eq!(lefts, [128, 146, 164, 182, 200]);
    assert_eq!(rights, [200, 200, 200, 200, 200]);
    assert_eq!(
        dim_box(rect, 10_000)[2] - dim_box(rect, 10_000)[0],
        0,
        "full sharpness: nothing left to shade"
    );
    // The clip mask's authored width is the box's width: the part of the blade past the cut.
    let widths: Vec<i32> = ratios
        .iter()
        .map(|bp| {
            let box_words = dim_box(rect, *bp);
            box_words[2] - box_words[0]
        })
        .collect();
    assert_eq!(widths, [72, 54, 36, 18, 0], "tip minus kept on the measured bar");
    assert_eq!(dim_box(rect, 0)[1], 38, "the bar's own rect rows");
    assert_eq!(dim_box(rect, 0)[3], 166);
    // A bar laid out wider than the measured one cuts the same fraction of its blade, and the
    // shadow never runs past the bar's rect.
    let wide = [100, 40, 268, 168];
    assert_eq!(dim_box(wide, 0)[0], 123, "18 * 168 / 128 = 23 px in from the rect's left");
    assert_eq!(dim_box(wide, 10_000)[2], 218);
    assert_eq!(dim_box(wide, 10_000)[0], 218);
    assert!(dim_box(wide, 10_000)[2] <= wide[2]);
}

#[test]
fn the_shade_clip_nodes_fit_the_grown_tree_exactly() {
    // r56's tree: the mask as the controller's first child over bar, glow and edge (accepted), and
    // the shade as the mask's sibling - anchor, its clip mask, the copy inside that clip - so the
    // copy is walked last, outside the accepted scissor, and inside the clip's scope.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, edge) = gauge_tree(&mut fake, true);
    let (mask, anchor, clip, dim) = (0x7500u32, 0x7600u32, 0x7700u32, 0x7800u32);
    for node in [mask, anchor, clip, dim] {
        fake.put(node + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    }
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, anchor);
    fake.put(mask + NODE_PARENT, root);
    fake.put(anchor + NODE_CHILD, clip);
    fake.put(anchor + NODE_SIBLING, 0);
    fake.put(anchor + NODE_PARENT, root);
    fake.put(clip + NODE_CHILD, dim);
    fake.put(clip + NODE_SIBLING, 0);
    fake.put(clip + NODE_PARENT, anchor);
    fake.put(dim + NODE_CHILD, 0);
    fake.put(dim + NODE_SIBLING, 0);
    fake.put(dim + NODE_PARENT, clip);
    for node in [sharp, efx, edge] {
        fake.put(node + NODE_PARENT, mask);
    }
    fake.put(root + NODE_CHILD, mask);
    fake.put(root + NODE_SIBLING, 0);
    // Seven tree nodes, so main+0x5456e4 walks eight entries and the map has to be eight words.
    assert_eq!(count_nodes(&fake, root), (7, false));
    assert_eq!(7 + 1, 8, "the array length the pass indexes with");
    // Records, from the walker port: mask enter, bar, glow and edge enter/exit, mask exit, anchor
    // enter, clip enter, copy enter/exit, clip exit, anchor exit, terminator = 15, and the block
    // sized for 7 objects is (7 << 4) | 8 = 120 bytes = 15 records, so they fit exactly.
    let plan = draw_plan(&fake, root);
    assert_eq!(
        plan.kinds[..plan.count as usize],
        [0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1, 2]
    );
    assert_eq!(plan.count, 15);
    // The layering facts the chain is built for: the copy's enter/exit pair (records 11 and 12) sits
    // after the accepted mask's exit record (record 8), and inside the clip mask's enter/exit pair
    // (records 10 and 13), which is what clips it.
    assert_eq!(
        (
            plan.kinds[7],
            plan.kinds[8],
            plan.kinds[9],
            plan.kinds[10],
            plan.kinds[11],
            plan.kinds[12]
        ),
        (1, 0, 0, 0, 1, 1),
        "mask exit, anchor enter, clip enter, copy enter, copy exit, clip exit"
    );
    assert!(!plan.overflow && (plan.count as usize) <= ENTRIES);
    assert_eq!(cache_layout(7, 16).2, 120);
    assert_eq!(120 / 8, 15);
    assert_eq!(plan.count * 8, cache_layout(7, 16).2);
    // With the edge glyph hidden, as the looping tiers keep it, the plan is 13 records: 16 bytes of
    // the grown area go spare.
    fake.put(edge + NODE_FLAGS, NODE_DRAWABLE);
    let looping = draw_plan(&fake, root);
    assert_eq!(looping.count, 13);
    assert_eq!(looping.count * 8, 104);
    assert!(looping.count * 8 <= cache_layout(7, 16).2);
}
