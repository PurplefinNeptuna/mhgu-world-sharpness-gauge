//! Host fixtures for the gauge decisions. No game, no ARM, no Forge.
use sharpness_gauge_core::{
    CONTEXT_CLIP, CONTROLLER_CACHE, CONTROLLER_FLAGS, CONTROLLER_OWNER, CONTROLLER_ROOT,
    CONTROLLER_ROW, FIELDS, FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER, Gauge,
    HEAD_WORDS, INPUT_FLAGS, KEY_FIELDS, Local, MAX_NODES, MAX_SHARPNESS,
    Memory, NODE_CHILD, NODE_DRAWABLE, NODE_EFFECTIVE_RECT, NODE_FLAGS, NODE_KIND,
    NODE_RECT, NODE_SIBLING, NODE_TEXTURE_RECT, NODE_VISIBLE, NODE_WORDS,
    PLAYER_ARRAY, SCAN_LIMIT, SESSION_CELL, SESSION_LOCAL_INDEX, SHARPNESS,
    SHARPNESS_TYPE, SPLAYER_CELL, basis_points, fill_ratio, gauge_of,
    is_sharp_animation, local_player, ratio_of_words, read_gauge, sample_controller,
    scan_walk,
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
    assert_eq!(words[25], 0x7000);
    let first = HEAD_WORDS + NODE_WORDS;
    assert_eq!(words[HEAD_WORDS], 0x7000);
    assert_eq!(words[first], 0x7100);
    assert_eq!(words[first + 11..first + 15], [0, 280, 88, 304]);
    // The change detector covers the gauge trio, the controller, the player and
    // the first node address, and never reaches past the copied sample.
    assert!(KEY_FIELDS.iter().all(|index| *index < words.len()));
    for live in [FIELD_CURRENT, FIELD_MAXIMUM, FIELD_TIER, 0, 11, HEAD_WORDS] {
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
