//! Host fixtures for the gauge decisions. No game, no ARM, no Forge.
use sharpness_gauge_core::{
    band_bp, count_nodes, cache_layout, crop_span, Crop, relocate_prefix, cut_box, dim_box, draw_plan, is_sharp_animation, local_player,
    drawn_rect, mask_box, read_gauge, sample_controller, span_box, tier_band, window_matches,
    CLIP_HEIGHT, CLIP_TOP,
    CONTROLLER_ROW, ENTRIES, Gauge, Local,
    MAX_HUNTERS, MAX_SHARPNESS, Memory, NODE_CHILD, NODE_DRAWABLE, NODE_FLAGS,
    NODE_PARENT, NODE_SIBLING, NODE_VISIBLE,
    PLAYER_ARRAY, SCAN_LIMIT, SESSION_CELL, SESSION_LOCAL_INDEX, SHARPNESS,
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

/// A plain `current * 10_000 / maximum` bar, clamped, in basis points: the reference for a gauge
/// whose points sit in one band, which is what a tier rule answering 0 for any points gives.
fn ratio(current: i32, maximum: i32) -> Option<u32> {
    if maximum <= 0 {
        return None;
    }
    Some(((current as i64 * 10_000 / maximum as i64).clamp(0, 10_000)) as u32)
}

fn tier_of_row(row: &[u16; 7]) -> impl Fn(i32) -> Option<u32> + '_ {
    // main+0xf4794: the tier is how many thresholds are strictly below the points.
    move |points| Some(row.iter().filter(|threshold| (**threshold as i32) < points).count() as u32)
}

/// The kireaji row the tester's log weapon used, and its maximum without a Sharpness skill.
const LOG_ROW: [u16; 7] = [90, 130, 240, 310, 370, 450, 450];

/// The brief's example weapon: great sword level id 149, kireaji row 119. Its maximum is 400
/// with no skill, 425 with Sharpness+1 and 450 with Sharpness+2 (`min(level * 25 + 150, 450)`).
const EXAMPLE_ROW: [u16; 7] = [30, 100, 250, 330, 410, 430, 450];

#[test]
fn the_log_samples_refill_the_bar_at_every_tier() {
    // The verbatim samples of one controller in `runs/r42-eden-2026-10-30-run2.txt`, with kireaji
    // row 10 and maximum 150: each point value fills the bar inside its own tier's band.
    let tier_of = tier_of_row(&LOG_ROW);
    let fill = |points| band_bp(points, 150, &tier_of);
    assert_eq!(fill(150), Some(10_000)); // tier 2, band 130..150
    assert_eq!(fill(131), Some(500)); // one point into that band
    assert_eq!(fill(130), Some(10_000)); // tier 1, band 90..130: the threshold itself is its top
    assert_eq!(fill(100), Some(2_500));
    assert_eq!(fill(90), Some(10_000)); // tier 0, band 0..90
    assert_eq!(fill(8), Some(888));
    assert_eq!(fill(0), Some(0));
    // The tiers those samples logged, from the same rule.
    for (points, tier) in [(150, 2), (131, 2), (130, 1), (100, 1), (90, 0), (8, 0)] {
        assert_eq!(tier_of(points), Some(tier), "{points} is tier {tier}");
    }
}

#[test]
fn the_top_band_is_clipped_to_the_maximum_so_a_buff_reads_full() {
    // Sharpness+1 and +2 raise mMaxSharpness, so the top band ends partway through the tier's
    // points. Without that clip the top band of a buffed weapon could never read full.
    let tier_of = tier_of_row(&EXAMPLE_ROW);
    assert_eq!(band_bp(400, 400, &tier_of), Some(10_000)); // no skill: top tier 4 blue
    assert_eq!(band_bp(425, 425, &tier_of), Some(10_000)); // Sharpness+1: top tier 5 white
    assert_eq!(band_bp(450, 450, &tier_of), Some(10_000)); // Sharpness+2: top tier 6 purple
    // A fresh weapon at a buffed maximum is full in every tier below it too.
    for maximum in [400, 425, 450] {
        for points in 0..=maximum {
            assert_eq!(band_bp(points, maximum, &tier_of).is_some(), true, "{points}/{maximum}");
        }
    }
    // A buffed maximum stops partway through a band: at 425 the top band is 410..425, so one
    // point above its floor is a tenth of the bar; at 450 tier 6 runs 430..450.
    assert_eq!(band_bp(411, 425, &tier_of), Some(666), "1 of 15 points");
    assert_eq!(band_bp(430, 425, &tier_of), Some(10_000), "430 is above the maximum, draws full");
    assert_eq!(band_bp(431, 450, &tier_of), Some(500), "1 of 20 points");
    // The brief's own pair: at maximum 450, 430 is the top of tier 5 and 410 the top of tier 4.
    assert_eq!(band_bp(430, 450, &tier_of), Some(10_000));
    assert_eq!(band_bp(410, 450, &tier_of), Some(10_000));
    // The bands of the no-skill maximum, from the tier rule.
    assert_eq!(tier_band(380, 400, &tier_of), Some((330, 400)), "top band clipped to 400");
    assert_eq!(tier_band(300, 400, &tier_of), Some((250, 330)), "a band below the maximum");
}

#[test]
fn a_zero_width_top_band_is_never_landed_in() {
    // 58 kireaji rows end `.., 450, 450`, so their tier 6 band is zero-width and the tier rule
    // never lands in it: the points cannot pass 450, and 450 is tier 5, where it reads full.
    let row = [30u16, 100, 250, 330, 410, 450, 450];
    let tier_of = tier_of_row(&row);
    assert_eq!(tier_of(450), Some(5));
    assert_eq!(band_bp(450, 450, &tier_of), Some(10_000));
    assert_eq!(tier_band(450, 450, &tier_of), Some((410, 450)));
    for points in 0..=450 {
        assert_ne!(tier_of(points), Some(6), "{points} cannot be tier 6");
    }
}

#[test]
fn one_tier_for_every_points_is_the_plain_ratio() {
    // Every gate and lookup failure in the native function answers tier 0 for any points, and
    // then the band is the whole maximum, so the fill is plain `current / maximum`.
    let every_probe = |_points| Some(0u32);
    for (current, maximum) in [(150, 160), (0, 160), (160, 160), (7, 3)] {
        assert_eq!(
            band_bp(current, maximum, &every_probe),
            ratio(current, maximum),
            "{current}/{maximum}"
        );
    }
}

#[test]
fn a_probe_the_rule_cannot_answer_gives_no_ratio() {
    // A tier the plugin cannot read is the caller's unsupported state: no ratio, native gauge
    // alone. The upper search's first probe of 100..=400 is 250, the unreadable point here.
    let read = |points| (points <= 200).then_some((points / 40) as u32);
    assert_eq!(band_bp(100, 400, &read), None);
    // A search that reads every probe still answers.
    assert_eq!(band_bp(100, 200, &read), Some(5_250), "band 79..=119, 21 of 40 points");
}

#[test]
fn the_fill_needs_a_positive_maximum_and_clamps_its_points() {
    let tier_of = tier_of_row(&EXAMPLE_ROW);
    assert_eq!(band_bp(100, 0, &tier_of), None); // unverified maximum
    assert_eq!(band_bp(100, -1, &tier_of), None); // negative maximum is unsupported
    assert_eq!(band_bp(500, 400, &tier_of), Some(10_000)); // above maximum draws full
    assert_eq!(band_bp(-5, 400, &tier_of), Some(0)); // depletion floors at empty
    assert_eq!(band_bp(i32::MIN, 3, &tier_of), Some(0));
}

#[test]
fn every_probe_stays_within_the_maximum() {
    // The native function reads the weapon of the player it is called with, so a probe outside
    // the points the game can hand it is a call the game never makes.
    for (current, maximum) in [(0, 450), (1, 1), (7, 12), (449, 450), (450, 450), (-9, 100), (999, 380)] {
        let probes = std::cell::RefCell::new(Vec::new());
        let read = |points| {
            probes.borrow_mut().push(points);
            Some((points / 40) as u32)
        };
        band_bp(current, maximum, &read);
        let probes = probes.borrow();
        assert!(
            probes.iter().all(|points| *points >= 0 && *points <= maximum.max(0)),
            "{current}/{maximum} probed {probes:?}"
        );
        assert!(!probes.is_empty(), "{current}/{maximum} probed nothing");
    }
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
fn gauge_fields_are_signed_points() {
    let mut fake = Fake::default();
    fake.put(0x9c00 + SHARPNESS, 150u32);
    fake.put(0x9c00 + MAX_SHARPNESS, 160u32);
    assert_eq!(
        read_gauge(&fake, 0x9c00),
        Some(Gauge {
            current: 150,
            maximum: 160,
        })
    );
    fake.put(0x9c00 + SHARPNESS, (-7i32) as u32);
    let depleted = read_gauge(&fake, 0x9c00).expect("gauge");
    assert_eq!(depleted.current, -7, "signed points keep their sign through the read");
    assert_eq!(band_bp(depleted.current, depleted.maximum, &tier_of_row(&LOG_ROW)), Some(0));
    assert_eq!(read_gauge(&fake, 0), None);
}

/// Build the gauge controller fixture: animation row (id 4, packed counts 4 objects and 15
/// sequences, first object 29), the bar, glow and edge as siblings, and a local player with
/// 150/160 points.
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
    fake.put(controller + CONTROLLER_ROW, row);
    fake.put(controller + NODE_CHILD, root);
    fake.put(row, 4); // animation id
    fake.put(row + 4, 4 | (15 << 16)); // objects | sequences << 16
    fake.put(row + 0x0c, 29); // first object row
    fake.put(root + NODE_FLAGS, NODE_VISIBLE);
    fake.put(root + NODE_CHILD, sharp);
    fake.put(sharp + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(sharp + NODE_SIBLING, efx);
    fake.put(efx + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    fake.put(efx + NODE_SIBLING, edge);
    // The edge glyph is drawable but its visibility bit is clear, which is what the looping
    // tiers do.
    fake.put(edge + NODE_FLAGS, NODE_DRAWABLE);
    fake
}

#[test]
fn controller_sample_carries_the_player_and_its_gauge() {
    let fake = gauge_fixture();
    let sample = sample_controller(&fake, 0x6000, BASE).expect("gauge controller");
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
        })
    );
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
fn a_gauge_without_a_player_has_no_gauge_to_fill_from() {
    let mut fake = gauge_fixture();
    // Player absent (for example a non-quest state): the controller is still the gauge row.
    fake.put(BASE + SESSION_CELL, 0);
    let sample = sample_controller(&fake, 0x6000, BASE).expect("gauge controller");
    assert_eq!(sample.local, Local::default());
    assert_eq!(sample.gauge, None);
}

#[test]
fn the_node_count_stops_at_its_bound() {
    // A chain longer than the visit limit, reached through a controller, counts up to the limit
    // and reports the count as a floor.
    let mut fake = Fake::default();
    for index in 0..(SCAN_LIMIT as u32 + 16) {
        let node = 0xa000 + index * 0x10;
        fake.put(node + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
        fake.put(node + NODE_SIBLING, node + 0x10);
    }
    fake.put(0x9000 + NODE_CHILD, 0xa000);
    assert_eq!(count_nodes(&fake, 0x9000), (SCAN_LIMIT, true));
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
    // a 56-byte record area of 7 records, and a 248-byte allocation. A mask alone emits 9
    // records, 72 bytes, 16 past that area, so even that shape needs the block grown, here
    // for 4 objects. The full inserted tree is pinned by the_shade_clip_node_fits_the_grown_tree_exactly.
    assert_eq!(cache_layout(3, 16), (248, 192, 56));
    assert_eq!(cache_layout(4, 16), (264, 192, 72));
    assert_eq!(cache_layout(0, 16), (192, 192, 0));
    assert_eq!(cache_layout(3, 0), (56, 0, 56));
    assert_eq!(56 / 8, 7);
    assert_eq!(72 / 8, 9);
    // A mask alone, measured by the walker port: 9 records, exactly that area, and 16 bytes past
    // the row's own 56.
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
    // inline (byte 0x57 bit 1 is word bit 25, the drawable bit). 9 records is 72 bytes.
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
fn the_shade_clip_mask_box_is_built_from_its_own_matrix() {
    // The malformation this guards: the shade landing on the blade's left, over the filled part.
    // The builder
    // main+0xa4b390 reads the mask node's own transform and nothing else, so a mask's box is
    // `corner .. corner + authored` with the corner the node's translation at +0x40/+0x44. Every
    // mask from the class factory carries the gauge's corner (110,38): the accepted mask's box
    // 110 38 110+kept 166 from authored (kept, 128) pins the template's rows and that corner.
    let rect = [110, 38, 238, 166];
    let template = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 110.0, 38.0, 3.0, 1.0,
    ];
    let ratios = [0u32, 2500, 5000, 7500, 10_000];
    let authored_width = |bp: u32| {
        let shade = dim_box(rect, bp);
        (shade[2] - shade[0]).max(1)
    };
    // The authored pair with the translation left at the factory's corner, the bad case:
    let left: Vec<[i32; 4]> = ratios
        .iter()
        .map(|bp| mask_box(template, [authored_width(*bp), rect[3] - rect[1]]))
        .collect();
    assert_eq!(
        left.iter().map(|b| [b[0], b[2]]).collect::<Vec<_>>(),
        [[110, 182], [110, 164], [110, 146], [110, 128], [110, 111]],
        "the shade starts at the bar's left corner: the malformation this guards"
    );
    // The mask's own translation is the box's left edge, so the authored width spans from the cut
    // to the full-sharpness right edge: the box is the part of the blade past the cut.
    let shade_boxes: Vec<[i32; 4]> = ratios
        .iter()
        .map(|bp| {
            let shade = dim_box(rect, *bp);
            let mut matrix = template;
            matrix[12] = shade[0] as f32;
            matrix[13] = shade[1] as f32;
            mask_box(matrix, [authored_width(*bp), rect[3] - rect[1]])
        })
        .collect();
    assert_eq!(
        shade_boxes,
        [
            [128, 38, 200, 166],
            [146, 38, 200, 166],
            [164, 38, 200, 166],
            [182, 38, 200, 166],
            [200, 38, 201, 166],
        ],
        "the box is the blade past the cut, one px of nothing at full sharpness"
    );
    assert_eq!(authored_width(10_000), 1, "the zero-width scissor is never asked for");
    // The accepted cut's own box through the same fold at a kept width of 66.
    assert_eq!(mask_box(template, [66, 128]), [110, 38, 176, 166]);
}

#[test]
fn the_shade_clip_node_fits_the_grown_tree_exactly() {
    // The accepted tree: the mask as the controller's first child over bar, glow and edge, and
    // the shade's clip mask as the mask's sibling with the copy as its child, so the copy is walked
    // last, outside the accepted scissor, and inside the clip mask's scope.
    let mut fake = gauge_fixture();
    let (root, sharp, efx, edge) = gauge_tree(&mut fake, true);
    let (mask, clip, dim) = (0x7500u32, 0x7700u32, 0x7800u32);
    for node in [mask, clip, dim] {
        fake.put(node + NODE_FLAGS, NODE_VISIBLE | NODE_DRAWABLE);
    }
    fake.put(mask + NODE_CHILD, sharp);
    fake.put(mask + NODE_SIBLING, clip);
    fake.put(mask + NODE_PARENT, root);
    fake.put(clip + NODE_CHILD, dim);
    fake.put(clip + NODE_SIBLING, 0);
    // A sibling of the mask in the root's chain: the native sibling linker gives it the mask's
    // parent, so the pop from the clip's chain lands on the root and the walk ends there.
    fake.put(clip + NODE_PARENT, root);
    fake.put(dim + NODE_CHILD, 0);
    fake.put(dim + NODE_SIBLING, 0);
    fake.put(dim + NODE_PARENT, clip);
    for node in [sharp, efx, edge] {
        fake.put(node + NODE_PARENT, mask);
    }
    fake.put(root + NODE_CHILD, mask);
    fake.put(root + NODE_SIBLING, 0);
    // Six tree nodes, so main+0x5456e4 walks seven entries and the map has to be seven words.
    assert_eq!(count_nodes(&fake, root), (6, false));
    assert_eq!(6 + 1, 7, "the array length the pass indexes with");
    // Records, from the walker port: mask enter, bar, glow and edge enter/exit, mask exit, clip
    // enter, copy enter/exit, clip exit, terminator = 13, and the block sized for 6 objects is
    // (6 << 4) | 8 = 104 bytes = 13 records, so they fit exactly.
    let plan = draw_plan(&fake, root);
    assert_eq!(
        plan.kinds[..plan.count as usize],
        [0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 2]
    );
    assert_eq!(plan.count, 13);
    // The layering facts the chain is built for: the copy's enter/exit pair (records 10 and 11) sits
    // after the accepted mask's exit record (record 8), and inside the clip mask's enter/exit pair
    // (records 9 and 12), which is what clips it.
    assert_eq!(
        (
            plan.kinds[6],
            plan.kinds[7],
            plan.kinds[8],
            plan.kinds[9],
            plan.kinds[10],
            plan.kinds[11],
            plan.kinds[12]
        ),
        (1, 1, 0, 0, 1, 1, 2),
        "edge exit, mask exit, clip enter, copy enter, copy exit, clip exit, terminator"
    );
    assert!(!plan.overflow && (plan.count as usize) <= ENTRIES);
    assert_eq!(cache_layout(6, 16).2, 104);
    assert_eq!(104 / 8, 13);
    assert_eq!(plan.count * 8, cache_layout(6, 16).2);
    // With the edge glyph hidden, as the looping tiers keep it, the plan is 11 records: 16 bytes of
    // the grown area go spare.
    fake.put(edge + NODE_FLAGS, NODE_DRAWABLE);
    let looping = draw_plan(&fake, root);
    assert_eq!(looping.count, 11);
    assert_eq!(looping.count * 8, 88);
    assert!(looping.count * 8 <= cache_layout(6, 16).2);
}

#[test]
fn a_window_accepts_another_plugins_entry_hook_and_nothing_else() {
    let original = core::array::from_fn::<u8, 32, _>(|i| i as u8 + 1);
    assert!(window_matches(&original, &original));

    let mut hooked = original;
    hooked[..4].copy_from_slice(&0xe51f_f004u32.to_le_bytes());
    hooked[4..8].copy_from_slice(&0x0320_1000u32.to_le_bytes());
    assert!(window_matches(&hooked, &original));

    let mut other_patch = original;
    other_patch[0] ^= 1;
    assert!(!window_matches(&other_patch, &original));

    let mut changed_later = hooked;
    changed_later[8] ^= 1;
    assert!(!window_matches(&changed_later, &original));
}

#[test]
fn the_drawn_box_follows_the_drawn_scale_and_position() {
    let at = |scale: f32, x: f32, y: f32| [scale, 0.0, 0.0, scale, x, y];
    // The measured plain rect, then UIScale's default 0.75 around a moved corner.
    assert_eq!(drawn_rect(at(1.0, 110.0, 38.0), 0, false, [128, 128]), Some([110, 38, 238, 166]));
    assert_eq!(drawn_rect(at(0.75, 100.0, 20.0), 0, false, [128, 128]), Some([100, 20, 196, 116]));
    // Anchor modes: x centred, y ending at the origin.
    assert_eq!(drawn_rect(at(1.0, 100.0, 100.0), 1 | (2 << 2), false, [10, 20]), Some([95, 80, 105, 100]));
    // Nothing a flip, a shear or a zero scale could turn into a box.
    assert_eq!(drawn_rect(at(1.0, 0.0, 0.0), 0, true, [128, 128]), None);
    assert_eq!(drawn_rect([1.0, 0.5, 0.0, 1.0, 0.0, 0.0], 0, false, [128, 128]), None);
    assert_eq!(drawn_rect(at(0.0, 0.0, 0.0), 0, false, [128, 128]), None);
}

/// Row setup (main+0xa484c4) points each object's `+0x64` into the draw block's 192-byte prefix,
/// and the row update evaluates those tracks every frame, so a replaced block must carry them.
#[test]
fn track_pointers_follow_the_replaced_draw_block() {
    let (old, fresh, prefix) = (0x4801_9000, 0x4802_6000, cache_layout(3, 16).1);
    assert_eq!(relocate_prefix(old, old, prefix, fresh), Some(fresh));
    assert_eq!(relocate_prefix(old + 36, old, prefix, fresh), Some(fresh + 36));
    // An object with no tracks holds the cursor at the prefix's end.
    assert_eq!(relocate_prefix(old + prefix, old, prefix, fresh), Some(fresh + prefix));
    // The record area past the prefix, earlier memory and zero are not track slices.
    assert_eq!(relocate_prefix(old + prefix + 4, old, prefix, fresh), None);
    assert_eq!(relocate_prefix(old - 4, old, prefix, fresh), None);
    assert_eq!(relocate_prefix(0, old, prefix, fresh), None);
}

#[test]
fn a_crop_keeps_the_cut_part_of_the_sprite_at_its_own_place_and_texel_size() {
    // A quad 128 wide at x 110, scale 1, anchor 0, cut by the 110..238 rect at 50% fill.
    let window = [0.0, 280.0, 128.0, 304.0];
    let cut = 110 + cut_box([110, 38, 238, 166], 5_000)[2];
    let bar = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 128.0, window, f32::NEG_INFINITY, cut as f32).unwrap();
    assert_eq!(cut, 164);
    assert_eq!(bar.x, 110.0);
    assert!((bar.x + bar.x_scale * 128.0 - 164.0).abs() < 1e-4);
    assert!((bar.window[2] - 54.0).abs() < 1e-4 && bar.window[0] == 0.0 && bar.window[1] == 280.0 && bar.window[3] == 304.0);
    // The shade: the part between the cut and the full-sharpness tip, from the dim box.
    let shade = dim_box([110, 38, 238, 166], 5_000);
    let dim = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 128.0, window, shade[0] as f32, shade[2] as f32).unwrap();
    assert!((dim.x - 164.0).abs() < 1e-4);
    assert!((dim.x + dim.x_scale * 128.0 - shade[2] as f32).abs() < 1e-4);
    assert!((dim.window[0] - 54.0).abs() < 1e-4 && (dim.window[2] - (shade[2] - 110) as f32).abs() < 1e-4);
    // A whole span is the sprite unchanged.
    let whole = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 128.0, window, 0.0, 1280.0).unwrap();
    assert_eq!(whole, Crop { x_scale: 1.0, x: 110.0, window });
}

#[test]
fn a_centred_or_scaled_sprite_is_cut_at_the_same_canvas_columns() {
    // Centred anchor at 174, a quad 128 wide: canvas 110..238, kept 150..200.
    let c = crop_span([1.0, 0.0, 0.0, 174.0], 1, false, 128.0, [0.0, 0.0, 64.0, 8.0], 150.0, 200.0).unwrap();
    assert!((c.x - c.x_scale * 64.0 - 150.0).abs() < 1e-4 && (c.x + c.x_scale * 64.0 - 200.0).abs() < 1e-4);
    assert!((c.window[0] - 20.0).abs() < 1e-4 && (c.window[2] - 45.0).abs() < 1e-4);
    // A sprite drawn at half scale: canvas 110..174, cut at 142 keeps its first half.
    let h = crop_span([0.5, 0.0, 0.0, 110.0], 0, false, 128.0, [0.0, 0.0, 128.0, 8.0], 0.0, 142.0).unwrap();
    assert!((h.x - 110.0).abs() < 1e-4 && (h.x_scale - 0.25).abs() < 1e-6 && (h.window[2] - 64.0).abs() < 1e-4);
}

#[test]
fn an_empty_span_draws_nothing_and_an_unmodelled_sprite_is_left_alone() {
    let window = [0.0, 280.0, 128.0, 304.0];
    let empty = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 128.0, window, 238.0, 238.0).unwrap();
    assert_eq!(empty.x_scale, 0.0);
    let outside = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 128.0, window, 300.0, 400.0).unwrap();
    assert_eq!(outside.x_scale, 0.0);
    for (m, anchors, flipped, width) in [
        ([1.0, 0.0, 0.0, 110.0], 3, false, 128.0),
        ([1.0, 0.0, 0.0, 110.0], 0, true, 128.0),
        ([1.0, 0.1, 0.0, 110.0], 0, false, 128.0),
        ([1.0, 0.0, 0.1, 110.0], 0, false, 128.0),
        ([0.0, 0.0, 0.0, 110.0], 0, false, 128.0),
        ([1.0, 0.0, 0.0, 110.0], 0, false, 0.0),
        ([1.0, 0.0, 0.0, f32::NAN], 0, false, 128.0),
    ] {
        assert!(crop_span(m, anchors, flipped, width, window, 0.0, 164.0).is_none());
    }
}

#[test]
fn the_bar_quad_is_its_authored_88_not_its_128_layout_box() {
    // Layout rect 110..238, but the quad main+0xa4feb4 draws is the authored 88 from x 110: at full
    // sharpness the cut at 200 is past its end at 198, so nothing is trimmed.
    let rect = [110, 38, 238, 166];
    let window = [0.0, 280.0, 88.0, 304.0];
    let full = (110 + cut_box(rect, 10_000)[2]) as f32;
    let whole = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 88.0, window, f32::NEG_INFINITY, full).unwrap();
    assert_eq!(whole, Crop { x_scale: 1.0, x: 110.0, window });
    // At 50% the cut at 164 keeps 54 of the 88 texels at their own size: r68's scissor edge.
    let half = (110 + cut_box(rect, 5_000)[2]) as f32;
    let cut = crop_span([1.0, 0.0, 0.0, 110.0], 0, false, 88.0, window, f32::NEG_INFINITY, half).unwrap();
    assert!((cut.x + cut.x_scale * 88.0 - 164.0).abs() < 1e-4 && (cut.window[2] - 54.0).abs() < 1e-4);
}
