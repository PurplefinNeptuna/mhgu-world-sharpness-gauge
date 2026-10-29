//! Host fixtures for the gauge decisions. No game, no ARM, no Forge.
use sharpness_gauge_core::{
    boundary, kept, prototype_ratio,
    BAR_INK_LEFT, CLIP_HEIGHT, CLIP_TOP, mask_box,
    CMD_TBL_ID, CONTEXT_CLIP, CONTROLLER_CACHE, CONTROLLER_FLAGS, CONTROLLER_OWNER, CONTROLLER_ROOT,
    CONTROLLER_ROW, ENTRIES, FIELDS, FIELD_CURRENT, FIELD_DRAWABLE, FIELD_MAXIMUM, FIELD_TIER, FIELD_VISITED,
    FIELD_VISIBLE, Gauge, Sample,
    HEAD_WORDS, INPUT_FLAGS, KEY_FIELDS, Local, MAX_NODES, MAX_SHARPNESS,
    Memory, NODE_CHILD, NODE_DRAWABLE, NODE_EFFECTIVE_RECT, NODE_FLAGS, NODE_KIND,
    NODE_PARENT, NODE_RECT, NODE_SIBLING, NODE_TEXTURE_RECT, NODE_VISIBLE, NODE_WORDS,
    PLAYER_ARRAY, SCAN_LIMIT, SESSION_CELL, SESSION_LOCAL_INDEX, SHARPNESS,
    SHARPNESS_TYPE, SPLAYER_CELL, basis_points, draw_plan, fill_ratio, gauge_of,
    is_sharp_animation, local_player, ratio_of_words, read_gauge, sample_controller,
    scan_walk, count_nodes, cache_layout, SHARP_OBJECTS, SPRITE_WIDTH,
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
    assert_eq!(fill_ratio(Gauge {
        current: 150,
        maximum: 160,
        tier: 3
    }), Some(0.9375));
}

#[test]
fn head_field_indexes_point_at_their_names() {
    // The r9 log printed three tree nodes for visited=3 and sized the draw cache
    // from a word that was the truncated flag, not the drawable count. The head
    // block is positional, so pin every walk field to the value it must carry.
    let mut sample = Sample::default();
    sample.walk.visited = 21;
    sample.walk.visible = 22;
    sample.walk.drawable = 23;
    sample.walk.truncated = true;
    let words = sample.words();
    assert_eq!(words[FIELD_VISITED], 21);
    assert_eq!(words[FIELD_VISIBLE], 22);
    assert_eq!(words[FIELD_DRAWABLE], 23);
    assert_eq!(words[FIELD_DRAWABLE + 1], 1);
    assert_eq!(HEAD_WORDS, 26);
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
    assert_eq!(local_player(&fake, BASE), Some(Local {
        player: 0x9c00,
        hunter_index: 2
    }));

    // Hunter index outside 0..3 rejects the whole chain.
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 7);
    assert_eq!(local_player(&fake, BASE), None);

    // A null session cell (no quest state) is unsupported, not a crash.
    fake.put(BASE + SESSION_CELL, 0);
    assert_eq!(local_player(&fake, BASE), None);

    // An empty slot in the player array is unsupported.
    fake.put(BASE + SESSION_CELL, 0x3000);
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 3);
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
/// siblings, a draw context carrying the live clip, and a local player with
/// 150/160 points at tier 2.
fn gauge_fixture() -> Fake {
    let mut fake = Fake::default();
    let (row, controller, root, sharp, efx, edge, player, context) =
        (0x5000u32, 0x6000u32, 0x7000u32, 0x7100u32, 0x7200u32, 0x7300u32, 0x9c00u32, 0x9000u32);
    fake.put(BASE + SESSION_CELL, 0x3000);
    fake.put(0x3000 + SESSION_LOCAL_INDEX, 0);
    fake.put(BASE + SPLAYER_CELL, 0x4000);
    fake.put(0x4000 + PLAYER_ARRAY, player);
    fake.put(player + SHARPNESS, 150);
    fake.put(player + MAX_SHARPNESS, 160);
    fake.put(player + SHARPNESS_TYPE, 2);
    fake.put(player + INPUT_FLAGS, 0x11);
    fake.put(player + CMD_TBL_ID, 0x2003);
    fake.put(controller + CONTROLLER_ROW, row);
    fake.put(controller + CONTROLLER_ROOT, root);
    fake.put(controller + CONTROLLER_OWNER, 0x8000);
    fake.put(controller + CONTROLLER_CACHE, 0x8800);
    fake.put(controller + CONTROLLER_FLAGS, 1); // rebuild flag set
    fake.put(row, 4); // animation id
    fake.put(row + 4, 4 | (15 << 16)); // objects | sequences << 16
    fake.put(row + 0x0c, 29); // first object row
    fake.put(context + CONTEXT_CLIP, 100);
    fake.put(context + CONTEXT_CLIP + 4, 30);
    fake.put(context + CONTEXT_CLIP + 8, 520);
    fake.put(context + CONTEXT_CLIP + 12, 360);
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
fn controller_sample_carries_the_gauge_state() {
    let fake = gauge_fixture();
    let sample = sample_controller(&fake, 0x6000, 0x9000, BASE).expect("gauge controller");
    assert_eq!((sample.animation, sample.objects, sample.sequences), (4, 4, 15));
    assert_eq!(sample.first_object, 29);
    assert_eq!((sample.owner, sample.cache, sample.cache_flags), (0x8000, 0x8800, 1));
    assert_eq!(sample.local, Local {
        player: 0x9c00,
        hunter_index: 0
    });
    assert_eq!(sample.input_flags, 0x11);
    assert_eq!(sample.cmd_tbl_id, 0x2003);
    assert_eq!(sample.context, 0x9000);
    assert_eq!(sample.clip, [100, 30, 520, 360]);
    assert_eq!(
        sample.gauge,
        Some(Gauge {
            current: 150,
            maximum: 160,
            tier: 2
        })
    );
    assert_eq!(sample.walk.visited, 4);
    assert_eq!((sample.walk.visible, sample.walk.drawable), (3, 3));
    assert_eq!(sample.walk.nodes[1].address, 0x7100);
    assert_eq!(sample.walk.nodes[1].rect, [110, 38, 198, 62]);
    assert_eq!(sample.walk.nodes[1].effective, [110, 38, 198, 62]);
    // The atlas rectangle of Txt_sharp recorded from the archive: 0,280,88,304.
    assert_eq!(sample.walk.nodes[1].texture, [0, 280, 88, 304]);
    let words = sample.words();
    assert_eq!(words.len(), FIELDS.len());
    assert_eq!(gauge_of(&words), sample.gauge);
    assert_eq!(ratio_of_words(&words), Some(9_375));
    assert_eq!(words[FIELD_CURRENT] as i32, 150);
    assert_eq!(words[FIELD_MAXIMUM] as i32, 160);
    assert_eq!(words[FIELD_TIER], 2);
    assert_eq!(words[HEAD_WORDS - 1], 0x2003);
    let first = HEAD_WORDS + NODE_WORDS;
    assert_eq!(words[HEAD_WORDS], 0x7000);
    assert_eq!(words[first], 0x7100);
    assert_eq!(words[first + 11..first + 15], [0, 280, 88, 304]);
    // The change detector covers the gauge trio, the controller, the player and
    // the first node address, and never reaches past the copied sample.
    assert!(KEY_FIELDS.iter().all(|index| *index < words.len()));
    for live in [FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER, FIELD_VISITED, 0, 11, HEAD_WORDS] {
        assert!(KEY_FIELDS.contains(&live));
    }
    assert_ne!((words[0], words[11], words[12]), (0, 0, 0));
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
    // Shape proven by the r9 Eden log: bar 0x488aa9b0, glow 0x4872c300 and the
    // edge are siblings whose parent is the controller 0x48898ec0, and the walk
    // closes when a sibling comes back to that root. The gate set in gui.rs is
    // written against this shape, so pin it here.
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

    // The r10 log closed the chain with the edge glyph's sibling at 0 and popped
    // through its parent, which is the root. Both closings end the walk.
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
    assert_eq!(sample_controller(&fake, 0x6100, 0x9000, BASE), None);
    // A null row pointer is unsupported state, not a fault.
    fake.put(0x6200 + CONTROLLER_ROW, 0);
    assert_eq!(sample_controller(&fake, 0x6200, 0x9000, BASE), None);
}

#[test]
fn a_gauge_without_a_player_reports_the_tree_only() {
    let mut fake = gauge_fixture();
    // Player absent (for example a non-quest state): the sample records the
    // tree and leaves the gauge fields empty, so the ratio is unsupported.
    fake.put(BASE + SESSION_CELL, 0);
    let sample = sample_controller(&fake, 0x6000, 0x9000, BASE).expect("gauge controller");
    assert_eq!(sample.local, Local::default());
    assert_eq!(sample.gauge, None);
    assert_eq!(sample.input_flags, 0);
    assert_eq!(sample.walk.drawable, 3);
    assert_eq!(ratio_of_words(&sample.words()), None);
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
    assert_eq!(scan.nodes[MAX_NODES - 1].address, 0xa000 + (MAX_NODES as u32 - 1) * 0x10);
    // A dangling link is recorded as truncation, never followed.
    let mut broken = Fake::default();
    broken.put(0xb000 + NODE_FLAGS, NODE_VISIBLE);
    broken.put(0xb000 + NODE_CHILD, 0xcccc_cccc);
    let scan = scan_walk(&broken, 0xb000);
    assert_eq!(scan.visited, 1);
    assert!(scan.truncated);
}

#[test]
fn the_prototype_fill_shares_one_boundary_column() {
    // The rects the r3 run measured on the Long Sword gauge. The bar's drawn ink is measured on
    // the rect the node reports: it starts 14 px from the rect's left (124 px, left of the hilt
    // glyph) and runs 101 px to 225, the r33 100% box that photographed the whole blade with no
    // cut. In sprite columns that is 10 in and 70 long at the rect's 128 px.
    let sword = [110, 38, 238, 166];
    let effect = [116, 32, 244, 160];
    for (ratio, column) in [(0u32, 120), (2_500, 137), (5_000, 155), (10_000, 190)] {
        assert_eq!(boundary(sword, ratio, 88), column);
        assert_eq!(kept(sword, column), [110, 38, column, 166]);
        assert_eq!(kept(effect, column), [116, 32, column.max(116), 160]);
    }
    for (ratio, column) in [(0u32, 124), (2_500, 149), (5_000, 174), (10_000, 225)] {
        assert_eq!(boundary(sword, ratio, 128), column);
    }
    // Ratio 0 keeps a non-zero clip (14 px at the rect's width), so the degenerate zero-size
    // scissor never has to be trusted, and it stops left of the hilt glyph the two 0%
    // screenshots show. Ratio 1 covers the whole blade. The clip rows start at the effect
    // glyph's top and cover the bar's rows.
    assert_eq!(boundary(sword, 0, 128) - sword[0], BAR_INK_LEFT * 128 / SPRITE_WIDTH);
    assert_eq!(boundary(sword, 10_000, 128), 225);
    assert_eq!((CLIP_TOP, CLIP_HEIGHT), (-6, 32));
    assert_eq!(kept([200, 0, 100, 10], 260), [200, 0, 200, 10]);
    assert_eq!(
        (0..200).step_by(20).map(prototype_ratio).collect::<Vec<_>>(),
        [0, 2_500, 5_000, 10_000, 0, 2_500, 5_000, 10_000, 0, 2_500]
    );
}

#[test]
fn the_mask_box_covers_the_bar_and_the_effect() {
    let sword = [110, 38, 238, 166];
    // Top is 32 and the height 32: the bar's ink rows (+6..+17) and the effect glyph's
    // offset rows (-6..+25) both fall inside. Width is never zero.
    for (ratio, box_) in [
        (0u32, [110, 32, 10, 32]),
        (2_500, [110, 32, 27, 32]),
        (5_000, [110, 32, 45, 32]),
        (10_000, [110, 32, 80, 32]),
    ] {
        assert_eq!(mask_box(sword, ratio, 88), box_);
        assert!(box_[2] > 0 && box_[3] > 0);
    }
    assert_eq!(mask_box(sword, 10_000, 88)[2], boundary(sword, 10_000, 88) - sword[0]);
    assert_eq!(mask_box(sword, 0, 88)[2], BAR_INK_LEFT);
    assert_eq!(mask_box(sword, 0, 128)[2], 14);
    assert_eq!(mask_box(sword, 10_000, 128)[2], 115);
}
#[test]
fn the_fill_track_scales_with_the_live_bar_width() {
    // A 176-wide bar is the same art at double scale: the inset and the track double.
    let sword = [110, 38, 238, 166];
    assert_eq!(boundary(sword, 10_000, 176), 110 + 20 + 140);
    assert_eq!(boundary(sword, 0, 176), 110 + 20);
    assert_eq!(boundary(sword, 5_000, 176), 110 + 20 + 70);
    assert_eq!(mask_box(sword, 10_000, 176)[2], 160);
}

/// The gauge tree as the runs show it, with every link the walker follows: the
/// bar, glow and edge are siblings whose parent is the walk root, and the edge
/// closes the chain with a zero sibling (r10/r11 Eden logs).
fn gauge_tree(fake: &mut Fake, edge_visible: bool) -> (u32, u32, u32, u32) {
    let (root, sharp, efx, edge) = (0x7000u32, 0x7100u32, 0x7200u32, 0x7300u32);
    fake.put(sharp + NODE_PARENT, root);
    fake.put(efx + NODE_PARENT, root);
    fake.put(edge + NODE_PARENT, root);
    fake.put(edge + NODE_SIBLING, 0);
    // The r10 log prints the edge glyph as `child=0 sibling=0 parent=controller`,
    // so the gauge tree has nothing below it.
    fake.put(edge + NODE_CHILD, 0);
    if edge_visible {
        fake.put(edge + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    }
    (root, sharp, efx, edge)
}

#[test]
fn the_walk_plan_closes_every_drawable_it_opens() {
    // The port of main+0xa494a0 over the tree before any insertion: enter and exit
    // for the bar and the glow, nothing for the hidden edge glyph, then the
    // terminator. 5 records is the 40 bytes the r9-r11 logs show, inside the 56-byte
    // record area main+0xa49b28 sizes for this row (2 records spare).
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
    // root+0x80 = alloc + sequences * 12. The r17 run read this row's sizing pair at row+8 as
    // 3 objects and 16 sequences, and published minus alloc is 192 = 16 * 12 in all three runs
    // (r17: 0x47fe7da0 - 0x47fe7ce0), so the record area is (3 << 4) | 8 = 56 bytes = 7
    // records and the allocation is 248. That pair is also why r17's gate turned the insertion
    // off (stage 48: 3 is not 3 + 1), and it restores the r14 cache claim: the inserted tree's
    // 9 records are 72 bytes, 16 past this row's 56.
    assert_eq!(cache_layout(3, 16), (248, 192, 56));
    assert_eq!(cache_layout(4, 16), (264, 192, 72));
    assert_eq!(cache_layout(0, 16), (192, 192, 0));
    assert_eq!(cache_layout(3, 0), (56, 0, 56));
    assert_eq!(56 / 8, 7);
    assert_eq!(72 / 8, 9);
    // The tree the insertion produces, measured by the walker port: 9 records, exactly the
    // grown area, and 16 bytes past the row's own 56 - which is r14's overflow.
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
    // r14 added a fourth node, the scissor mask, and left the array at 4: that pass then
    // walks 5 entries through a 4-entry array. The r14 Eden run of 2026-10-23 ended in
    // that loop, with a freed block's fill (0x727272) as the object pointer, in the
    // teardown after the quest. Pinned so an insertion is never shipped without the map.
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
    // inline (byte 0x57 bit 1 is word bit 25, the drawable bit the r10 log shows as
    // 0x2040000). 9 records is 72 bytes, and the r17 run measured this row's record area at
    // (3 << 4) | 8 = 56, so the inserted tree writes 16 bytes past it: r14's overflow, and the
    // reason grow_cache rebuilds the block for 4 objects (72, exactly this plan).
    fake.put(edge + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    let full = draw_plan(&fake, root);
    assert_eq!(full.kinds[..full.count as usize], [0, 0, 1, 0, 1, 0, 1, 1, 2]);
    assert_eq!(full.count * 8, 72);
    assert!(!full.overflow && (full.count as usize) <= ENTRIES);
}

#[test]
fn a_mask_left_open_never_stops_clipping() {
    // The r13 shape: the mask takes the bar as its child but keeps the glow and the
    // edge as siblings whose parent is the root, so the pop from the edge jumps to
    // the root and ends the walk. The mask gets an enter record and no exit record,
    // which is a scissor that never ends. Pinned so this shape cannot come back.
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
fn the_forced_ratios_step_the_box_from_the_rect_width() {
    // The width fed to mask_box is the node's rect width, measured from the tester's 100%
    // screenshots: r33's box (right 226, the 128 px rect's scale) shows the whole blade, and
    // r34's (right 190, the authored 88 px scale) cuts the blade near the tip.
    let bar = [110, 38, 238, 166];
    let rect_width = bar[2] - bar[0];
    let ratios: Vec<u32> = (0..160u32).step_by(20).map(prototype_ratio).collect();
    assert_eq!(ratios[..4].to_vec(), vec![0, 2_500, 5_000, 10_000]);
    assert_eq!(ratios[4..8].to_vec(), ratios[0..4].to_vec());
    let widths: Vec<i32> = ratios[..4].iter().map(|r| mask_box(bar, *r, rect_width)[2]).collect();
    assert_eq!(widths, vec![14, 39, 64, 115]);
    // The kept widths 14, 39, 64 and 115 px, i.e. the box's right edge at 124, 149, 174 and
    // 225 px: the empty step stops left of the hilt glyph, and 100% covers the blade.
    assert_eq!(widths.iter().map(|w| bar[0] + w).collect::<Vec<_>>(), vec![124, 149, 174, 225]);
    assert!(widths[0] > 0 && widths[0] < 18, "the empty step must stop left of the hilt glyph");
    assert!(bar[0] + widths[3] >= 225);
    // The authored 88 px feed r34 used is refuted: its 100% box ends at 190, inside the blade.
    assert!(mask_box(bar, 10_000, 88)[2] + bar[0] < 225);
}
