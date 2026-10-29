//! Sharpness gauge decisions. Native offsets, pointers and calls stay in the
//! plugin adapter; everything here is plain data so host tests can call it
//! without the game.
//!
//! Offsets and units are the Phase 0 records in
//! `work/modern-sharpness-gauge/PHASE0_RESEARCH.md`, verified against the
//! hash-checked main NSO (`b3935415499549adaf4c0e89822831daa175cfd456a7990791c79be3997645ef`)
//! and the Ghidra type records. Object offsets are never rebased by the image
//! base; main-relative code addresses are irrelevant to this crate.
#![no_std]

/// One checked native word read. The plugin implements it with the mapping
/// checks in `src/memory.rs`; tests implement it with a flat table.
pub trait Memory {
    fn word(&self, address: u32) -> Option<u32>;
}

// Local hunter index lives in the session singleton; the sPlayer array holds
// one player pointer per hunter at +0x1c + 4 * index. Proven native helpers:
// main+0x3f8398 reads the index, main+0x276278 (sPlayer::getMainPlayer) reads
// the array. Never obtain the index from sUserInfo.
pub const SESSION_CELL: u32 = 0x188c_a48;
pub const SESSION_LOCAL_INDEX: u32 = 0x30;
pub const SPLAYER_CELL: u32 = 0x188_5b58;
pub const PLAYER_ARRAY: u32 = 0x1c;
pub const PLAYER_STRIDE: u32 = 4;
pub const MAX_HUNTERS: u32 = 4;

// uPlayerQuest (per-player instance, class size 13104) keeps sharpness as
// signed points, not a percentage. uPlayerBase (class size 10048) keeps the
// tier index as u32, returned verbatim by getSharpnessType at main+0x26893c.
// The plan's riding flag at +5104 & 0x20 is disproved: that word is the tier.
pub const SHARPNESS: u32 = 10_100;
pub const MAX_SHARPNESS: u32 = 10_104;
pub const SHARPNESS_TYPE: u32 = 5_104;

// Animation row (nGUI::ANIM): +0 animation id u32; the word at +4 is two u16
// counts (objects low, sequences high, confirmed by the r1 run); +12 is the
// first object row index; +20 is a pointer to the first sequence block, not a
// row index, which is why it is not part of the identity. The gauge identity
// tuple is identical in every inspected language archive and in the HD copy, so
// identification never uses names, strings or a language prefix.
pub const SHARP_ANIMATION: u32 = 4;
pub const SHARP_OBJECTS: u32 = 4;
pub const SHARP_FIRST_OBJECT: u32 = 29;
pub const SHARP_SEQUENCES: u32 = 15;
pub const ROW_ID: u32 = 0x00;
pub const ROW_COUNTS: u32 = 0x04;
pub const ROW_FIRST_OBJECT: u32 = 0x0c;

// Animation controller: +0x68 tree root node, +0x80 draw cache, +0x8c rebuild
// flags (bit 0 asks the walker to rebuild), +0x9c animation row, +0xa0 owner.
pub const CONTROLLER_ROOT: u32 = 0x68;
pub const CONTROLLER_CACHE: u32 = 0x80;
pub const CONTROLLER_FLAGS: u32 = 0x8c;
pub const CONTROLLER_ROW: u32 = 0x9c;
pub const CONTROLLER_OWNER: u32 = 0xa0;

// GUI object node, as walked by the draw walker at main+0xa494a0, painted by
// prepaint at main+0xa4e628, and written by the two cGUIObjTexture setters at
// main+0xa4fb58 (rect) and main+0xa4faf0 (texture). Ghidra indexes `int *`
// parameters in WORDS, so a member written `param_1[0x38]` sits at byte
// offset 0x38 * 4 = 0xE0; explicit `param_1 + 0x54` arithmetic is a byte
// offset. The object's own fields:
//   +0x54 u32   bytes +0x54..+0x57: bit 0 visible (the walker skips the whole
//               subtree when clear), 0x0200_0000 drawable (byte 2 bit 5, and the
//               same word bit as byte 0x57 bit 1, which is the bit the walker
//               tests twice: to emit the enter record, and to emit a childless
//               node's exit record inline), 0x0001_0000 cleared by the consumer
//               after each exit record, 0x0000_4000 byte 2 (+0x56) bit 6 prepaint
//               skip
//   +0x33 u32   low nibble is the object kind; kinds 6 and 3 take the clip path
//   +0xE0..0xEC four signed ints: own screen rect (left, top, right, bottom)
//   +0xF0..0xFC four signed ints: that rect after prepaint intersects it with
//               the live clip, i.e. exactly what native code draws
//   +0x100 u32  texture id, +0x104 texture object, +0x10C..0x118 four ints: the
//               rectangle inside the atlas, which identifies the glyph drawn
//   +0x42 byte  & 0x30 draw-dirty flag, +0xD0 u32 bit 0 rect-dirty, bit 1 clip
//               applied
pub const NODE_FLAGS: u32 = 0x54;
pub const NODE_KIND: u32 = 0x33;
/// The authored size pair's width word, written by the layout pass through the DTI `Size`
/// setter (main+0xa4ba40) and read by gui.rs's stage 32 gate: 88x24 for the bar, 88x32 for the
/// glow. The r33 run measured the bar's transform x scale at 1.0, so this is the width the
/// sprite is drawn at, in pixels, from the rect's left: the ink columns are pixels 1:1.
pub const NODE_AUTHORED_WIDTH: u32 = 0xb4;
pub const NODE_RECT: u32 = 0xe0;
pub const NODE_EFFECTIVE_RECT: u32 = 0xf0;
pub const NODE_TEXTURE_ID: u32 = 0x100;
pub const NODE_TEXTURE_RECT: u32 = 0x10c;
pub const NODE_CHILD: u32 = 0x68;
pub const NODE_SIBLING: u32 = 0x6c;
pub const NODE_PARENT: u32 = 0x70;
pub const NODE_VISIBLE: u32 = 1;
pub const NODE_DRAWABLE: u32 = 0x0200_0000;

// The walker's second argument is the draw context. Its live scissor rectangle,
// which prepaint intersects every drawable with, sits at +0xd0..+0xdc.
pub const CONTEXT_CLIP: u32 = 0xd0;

// mInputFlags in uPlayerBase, the candidate for the riding state. Unproven: the
// r1 run and the decompilations do not locate a riding bit, so r2 only copies
// this word for comparison.
pub const INPUT_FLAGS: u32 = 5_156;

/// mCmdTblId in uPlayerBase, the weapon command table id. The DTI type record
/// puts it at +9388; it is the value the animation and command lookups key on,
/// so it names the equipped weapon in every log line.
pub const CMD_TBL_ID: u32 = 9_388;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gauge {
    /// mSharpness, signed sharpness points.
    pub current: i32,
    /// mMaxSharpness, signed sharpness points.
    pub maximum: i32,
    /// mSharpnessType, native tier index: 0 red, 1 orange, 2 yellow, 3 green,
    /// 4 blue, 5 white, 6 purple (mapping supplied by the tester on 2026-10-22).
    /// Each tier is a per-weapon segment of points, so a tier boundary is never a
    /// fixed percentage of the maximum.
    pub tier: u32,
}

pub fn read_gauge(memory: &dyn Memory, player: u32) -> Option<Gauge> {
    let current = memory.word(player.checked_add(SHARPNESS)?)? as i32;
    let maximum = memory.word(player.checked_add(MAX_SHARPNESS)?)? as i32;
    let tier = memory.word(player.checked_add(SHARPNESS_TYPE)?)?;
    Some(Gauge {
        current,
        maximum,
        tier,
    })
}

/// The plan requires a verified positive maximum before any division, then
/// clamps to `[0,1]`. Depletion below zero draws empty and an over-max value
/// draws full; a zero, negative or unreadable maximum is unsupported state and
/// yields None, which leaves the native HUD untouched.
pub fn fill_ratio(gauge: Gauge) -> Option<f32> {
    basis_points(gauge).map(|points| points as f32 / 10_000.0)
}

/// Same rule in integer form, so the runtime never formats a float:
/// `current * 10_000 / maximum` clamped to `0..=10_000`.
pub fn basis_points(gauge: Gauge) -> Option<u32> {
    if gauge.maximum <= 0 {
        return None;
    }
    let points = (gauge.current as i64).checked_mul(10_000)? / gauge.maximum as i64;
    Some(points.clamp(0, 10_000) as u32)
}

pub fn is_sharp_animation(animation: u32, objects: u32, first_object: u32, sequences: u32) -> bool {
    animation == SHARP_ANIMATION
        && objects == SHARP_OBJECTS
        && first_object == SHARP_FIRST_OBJECT
        && sequences == SHARP_SEQUENCES
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub address: u32,
    pub flags: u32,
    pub kind: u32,
    /// Own screen rectangle, the rectangle prepaint leaves after the scissor
    /// intersection, and the rectangle inside the texture atlas.
    pub rect: [i32; 4],
    pub effective: [i32; 4],
    pub texture: [i32; 4],
    /// child (+0x68), sibling (+0x6c), parent (+0x70): the links the scissor mask
    /// insertion walks, logged so a failed gate names the link it rejected.
    pub links: [u32; 3],
}

/// Bounded pre-order walk of a controller's node tree. It records counts and
/// the leading nodes; no pointer is retained past the call. The stack depth,
/// visit count and recorded node count are fixed, so a damaged tree cannot run
/// away.
/// Prototype fill ratios from the plan's Phase 2 step (0, 0.25, 0.5 and 1), in
/// basis points, each held for `PROTOTYPE_SECONDS` before the next one.
pub const PROTOTYPE_RATIOS: [u32; 4] = [0, 2_500, 5_000, 10_000];
pub const PROTOTYPE_SECONDS: u32 = 20;

pub fn prototype_ratio(seconds: u32) -> u32 {
    PROTOTYPE_RATIOS[((seconds / PROTOTYPE_SECONDS) as usize) % PROTOTYPE_RATIOS.len()]
}

/// Measured from the atlas, not authored. `Txt_sharp`'s rect is 88x24 and its sprite
/// frames (hud00 rows 280/304/328) draw ink only in columns 18..=79 and rows +6..=+17,
/// so the clip track starts 18 in from the rect's left and runs 62 columns: ratio 0
/// leaves all of the ink out, ratio 1 covers it. `CLIP_TOP`/`CLIP_HEIGHT` cover the
/// effect glyph, whose authored offset is (6,-6) with height 32. Measured 2026-10-22
/// from `HD_hud00_BM_NOMIP.tex` (sha256 bd5d1a3fcc77454ef8a7ed715dda1a76a72e5e0ee6413d4ddc6f550c7b89e857).
/// The authored sprite width the ink measurements are expressed against. The r33 run proved the
/// sprite fills the node's layout rect: its 100% box (right 226) shows the whole blade, and r34
/// fed the authored 88 px, whose 100% box (right 190) cuts the blade near the tip. So a sprite
/// column measures rect_width / SPRITE_WIDTH px, and the runtime feeds the rect width.
pub const SPRITE_WIDTH: i32 = 88;
/// Where the drawn bar starts and stops, in sprite columns, measured from the r35 run's four
/// photographs with that run's own logged box rights as the ruler (scale 1.4 image px per GUI px,
/// fitted on the 25% and 50% cuts at 149 and 174). The drawn ink runs 124..200 px: the 0% box
/// (124) photographs no blue at all, and the 100% box (225) photographs blue up to 200 with the
/// box 25 px past the blade, so the old track (124..225, 128 px) overshot the gauge by half again
/// its length and put the 25% and 50% steps at 27% and 63% of the blade. Ending at 204 px is
/// 201..204, the measured tip (200) with margin: overshoot lands on empty sprite, undershoot cuts
/// the tip, which is what r34's 190 px box photographed.
pub const BAR_INK_LEFT: i32 = 10;
pub const BAR_INK_TRACK: i32 = 55;
pub const CLIP_TOP: i32 = -6;
pub const CLIP_HEIGHT: i32 = 32;

/// The shared screen column the fill stops at. The hilt is the left edge. The kept part grows
/// from the left and the tip side is clipped. The column is measured on the bar's own rect, the
/// rectangle the node reports at +0xE0, because the runs show the sprite drawn across that rect:
/// at the rect's 128 px the track is 14 px in and 80 px long, which lands the steps at 124, 144,
/// 164 and 204 px: the whole measured gauge (124..200) in four linear steps. The effect glyph is clipped at the same column, because its rect
/// carries its `+6,-6` local offset.
pub fn boundary(sword: [i32; 4], ratio_bp: u32, bar_width: i32) -> i32 {
    let width = bar_width.max(1);
    let left = (BAR_INK_LEFT as i64 * width as i64 / SPRITE_WIDTH as i64) as i32;
    let track = (BAR_INK_TRACK as i64 * width as i64 / SPRITE_WIDTH as i64) as i32;
    sword[0] + left + (track as i64 * ratio_bp as i64 / 10_000) as i32
}

/// The box from two measured endpoints, in pixels from the bar rect's left: the box's right
/// edge is `start` at ratio 0 and `end` at ratio 10000, linear between them. Width is never
/// below 1 and never past the rect's right, so the degenerate zero-size scissor is never asked
/// for. This is the tester's r38 test control: the two endpoints are set from the overlay, and
/// the accepted pair is 129..201 px, i.e. 19 and 91 from the rect's left.
pub fn span_box(rect: [i32; 4], ratio_bp: u32, start: i32, end: i32) -> [i32; 4] {
    let width = (start + (end - start) * ratio_bp as i32 / 10_000).max(1);
    [
        rect[0],
        rect[1] + CLIP_TOP,
        width.min(rect[2] - rect[0]),
        CLIP_HEIGHT,
    ]
}

/// The kept rectangle for one sprite under a shared column: a sprite starting
/// past the column keeps a zero-width rect, which is the empty side, and a sprite
/// ending before it keeps its full width.
pub fn kept(rect: [i32; 4], column: i32) -> [i32; 4] {
    [
        rect[0],
        rect[1],
        rect[2].min(column).max(rect[0]),
        rect[3],
    ]
}

/// The scissor mask box for one ratio, measured in the bar's rect: its left is the rect's
/// left, its top is `CLIP_TOP` above the rect's top so the effect glyph's offset rows are
/// inside, its height is `CLIP_HEIGHT`, and its width runs from the rect's left to the shared
/// boundary. Width is never zero (14 px at ratio 0), so the degenerate mask size is never
/// asked for.
pub fn mask_box(rect: [i32; 4], ratio_bp: u32, bar_width: i32) -> [i32; 4] {
    let column = boundary(rect, ratio_bp, bar_width);
    [rect[0], rect[1] + CLIP_TOP, column - rect[0], CLIP_HEIGHT]
}

/// The node count of main+0xa47d2c: every node in the child and sibling chains below
/// `controller+0x68`, visibility ignored (that function tests neither bit). main+0xa38ffc
/// returns it plus one, and main+0x5456e4 uses that sum as the length of the instance's
/// object array at `+0xd8`, calling vtable slot 0x50 on every entry it walks. So the row's
/// object array is sized for the tree it was built with: the gauge row has 4 objects for
/// 3 tree nodes plus the root, and a tree with more nodes than the row has objects makes
/// that pass read past the array. Returns (count, truncated); truncated means the bound
/// was hit, and the count is then a floor.
pub fn count_nodes(memory: &dyn Memory, controller: u32) -> (u32, bool) {
    let mut count = 0u32;
    let mut truncated = false;
    let mut stack = [0u32; SCAN_STACK];
    let mut top = 0usize;
    let mut next = memory.word(controller.wrapping_add(NODE_CHILD)).unwrap_or(0);
    loop {
        while next != 0 {
            if count == SCAN_LIMIT {
                return (count, true);
            }
            count += 1;
            let child = memory.word(next.wrapping_add(NODE_CHILD)).unwrap_or(0);
            if child != 0 {
                if top == SCAN_STACK {
                    truncated = true;
                } else {
                    stack[top] = child;
                    top += 1;
                }
            }
            next = memory.word(next.wrapping_add(NODE_SIBLING)).unwrap_or(0);
        }
        if top == 0 {
            return (count, truncated);
        }
        top -= 1;
        next = stack[top];
    }
}

/// The draw-cache sizing of main+0xa49b28, in full:
/// `size = (objects << 4 | 8) + sequences * 12`, allocated through the root's vtable slot
/// 0x58 and stored at `root+0xa4`; the pointer the walker draws from, `root+0x80`, is
/// `alloc + sequences * 12`, and it is only written when objects is nonzero. So the record
/// area is `(objects << 4) | 8` bytes: 8 bytes per record, a drawable's enter and exit pair
/// takes 16, and the terminator takes 8. The r17 run measured this row's pair at row+8 as 3
/// objects and 16 sequences, with the published pointer 192 = 16 * 12 into the allocation, so
/// its record area is (3 << 4) | 8 = 56 bytes = 7 records: the unmodified tree's 5 fit with 2
/// to spare, and the tree with an inserted mask emits 9 = 72, i.e. 16 bytes past the end.
/// Grown to 4 objects the area is 72, exactly the 9 the inserted tree needs. The row's other
/// pair, at row+4, is 4 objects and 15 sequences; the sizer does not read it. Returns
/// (size, prefix, record_area).
pub fn cache_layout(objects: u32, sequences: u32) -> (u32, u32, u32) {
    let prefix = sequences * 12;
    if objects == 0 {
        return (prefix, prefix, 0);
    }
    let area = (objects << 4) | 8;
    (area + prefix, prefix, area)
}

pub const SCAN_STACK: usize = 16;
pub const SCAN_LIMIT: u32 = 64;
pub const MAX_NODES: usize = 6;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    pub visited: u32,
    pub visible: u32,
    pub drawable: u32,
    pub nodes: [Node; MAX_NODES],
    pub truncated: bool,
}

pub fn scan_walk(memory: &dyn Memory, root: u32) -> Walk {
    let mut out = Walk::default();
    let mut stack = [0u32; SCAN_STACK];
    let mut top = 0usize;
    let mut overflow = false;
    macro_rules! push {
        ($node:expr) => {{
            let node = $node;
            if node != 0 {
                if top == SCAN_STACK {
                    overflow = true;
                } else {
                    stack[top] = node;
                    top += 1;
                }
            }
        }};
    }
    push!(root);
    while top > 0 && out.visited < SCAN_LIMIT {
        top -= 1;
        let node = stack[top];
        let Some(flags) = memory.word(node.wrapping_add(NODE_FLAGS)) else {
            overflow = true;
            break;
        };
        let visible = flags & NODE_VISIBLE != 0;
        out.visited += 1;
        if visible {
            out.visible += 1;
        }
        if flags & NODE_DRAWABLE != 0 {
            out.drawable += 1;
        }
        if out.visited as usize <= MAX_NODES {
            let mut recorded = Node {
                address: node,
                flags,
                kind: memory.word(node.wrapping_add(NODE_KIND)).unwrap_or(0) & 0xf,
                rect: [0; 4],
                effective: [0; 4],
                texture: [0; 4],
                links: [0; 3],
            };
            recorded.links[0] = memory.word(node.wrapping_add(NODE_CHILD)).unwrap_or(0);
            recorded.links[1] = memory.word(node.wrapping_add(NODE_SIBLING)).unwrap_or(0);
            recorded.links[2] = memory.word(node.wrapping_add(NODE_PARENT)).unwrap_or(0);
            for index in 0..4 {
                recorded.rect[index] = memory
                    .word(node.wrapping_add(NODE_RECT + (index as u32) * 4))
                    .unwrap_or(0) as i32;
                recorded.effective[index] = memory
                    .word(node.wrapping_add(NODE_EFFECTIVE_RECT + (index as u32) * 4))
                    .unwrap_or(0) as i32;
                recorded.texture[index] = memory
                    .word(node.wrapping_add(NODE_TEXTURE_RECT + (index as u32) * 4))
                    .unwrap_or(0) as i32;
            }
            out.nodes[out.visited as usize - 1] = recorded;
        }
        // The sibling is pushed first so the child branch is taken next; the
        // child is only reachable when this node is visible, matching the
        // walker's skip of invisible subtrees.
        push!(memory
            .word(node.wrapping_add(NODE_SIBLING))
            .unwrap_or(0));
        if visible {
            push!(memory.word(node.wrapping_add(NODE_CHILD)).unwrap_or(0));
        }
    }
    out.truncated = overflow || top > 0 || out.visited >= SCAN_LIMIT;
    out
}

/// The draw cache the walker at main+0xa494a0 writes for a tree, as the entry kinds
/// in the order it writes them: 0 = enter (the consumer calls vtable 0x38 and 0x3c),
/// 1 = exit (it calls 0x40 and clears 0x0001_0000 in the node's flag word), 2 = the
/// terminator. Ported from the decompile at `main+0xa494a0` instruction for
/// instruction: the walk enters at `root+0x68`, descends `+0x68`, takes `+0x6c`
/// siblings, pops through `+0x70` when a sibling is 0, and ends when the next pointer
/// is the root. A drawable node with no child emits its exit entry inline (byte 0x57
/// bit 1 is word bit 25, the same drawable bit). Bounded by ENTRIES: a tree that
/// emits more entries than the row's cache can hold is reported as overflow, because
/// the walker writes with no bounds check of its own.
pub const ENTRIES: usize = 16;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub kinds: [u32; ENTRIES],
    pub count: u32,
    /// The walk ran past ENTRIES, past SCAN_LIMIT visits, or a read failed.
    pub overflow: bool,
}

fn emit(plan: &mut Plan, kind: u32) {
    if (plan.count as usize) < ENTRIES {
        plan.kinds[plan.count as usize] = kind;
    } else {
        plan.overflow = true;
    }
    plan.count += 1;
}

pub fn draw_plan(memory: &dyn Memory, root: u32) -> Plan {
    let mut out = Plan::default();
    let mut node = match memory.word(root.wrapping_add(NODE_CHILD)) {
        Some(node) if node != 0 => node,
        _ => {
            emit(&mut out, 2);
            return out;
        }
    };
    let mut popped = false;
    for _ in 0..=SCAN_LIMIT {
        let Some(flags) = memory.word(node.wrapping_add(NODE_FLAGS)) else {
            out.overflow = true;
            break;
        };
        let drawable = flags & NODE_DRAWABLE != 0;
        let mut next;
        if popped {
            if drawable {
                emit(&mut out, 1);
            }
            popped = false;
            next = memory.word(node.wrapping_add(NODE_SIBLING)).unwrap_or(0);
            if next == 0 {
                next = memory.word(node.wrapping_add(NODE_PARENT)).unwrap_or(0);
                popped = true;
            }
        } else if flags & NODE_VISIBLE == 0 {
            next = memory.word(node.wrapping_add(NODE_SIBLING)).unwrap_or(0);
            if next == 0 {
                next = memory.word(node.wrapping_add(NODE_PARENT)).unwrap_or(0);
                popped = true;
            }
        } else {
            if drawable {
                emit(&mut out, 0);
            }
            let child = memory.word(node.wrapping_add(NODE_CHILD)).unwrap_or(0);
            if child != 0 {
                next = child;
            } else {
                if drawable {
                    emit(&mut out, 1);
                }
                next = memory.word(node.wrapping_add(NODE_SIBLING)).unwrap_or(0);
                if next == 0 {
                    next = memory.word(node.wrapping_add(NODE_PARENT)).unwrap_or(0);
                    popped = true;
                }
            }
        }
        if next == root {
            break;
        }
        if next == 0 {
            out.overflow = true;
            break;
        }
        node = next;
    }
    emit(&mut out, 2);
    out
}

/// Local hunter's player object and the session's hunter index. The index is
/// reported even when the chain has no player, so a log line can tell an absent
/// player apart from an absent chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Local {
    pub player: u32,
    pub hunter_index: u32,
}

/// Local hunter's player object, or None when the quest/session state does not
/// provide one. Every pointer in the chain is validated by the Memory impl.
/// `base` is the loaded main module address: the two cells below are
/// main-relative, the player pointers they yield are absolute heap addresses.
pub fn local_player(memory: &dyn Memory, base: u32) -> Option<Local> {
    let session = memory.word(base.checked_add(SESSION_CELL)?)?;
    if session == 0 {
        return None;
    }
    let index = memory.word(session.checked_add(SESSION_LOCAL_INDEX)?)?;
    if index >= MAX_HUNTERS {
        return None;
    }
    let array = memory.word(base.checked_add(SPLAYER_CELL)?)?;
    if array == 0 {
        return None;
    }
    let player = memory.word(array.checked_add(PLAYER_ARRAY + index.checked_mul(PLAYER_STRIDE)?)?)?;
    (player != 0).then_some(Local {
        player,
        hunter_index: index
    })
}

/// One bounded observation copied out inside the hot hook. All values are
/// plain u32 words (signed fields are two's complement) so the hook performs
/// no allocation, formatting or file access. `FIELDS` names each word in the
/// same order as `words`, which is the log format and the ring-buffer layout.
pub const FIELDS: [&str; 134] = [
    "ctrl",     "row",     "owner",     "cache",
    "cache_flags",     "root",     "anim",     "objects",
    "sequences",     "first_object",     "hunter",     "player",
    "current",     "maximum",     "tier",     "input_flags",
    "context",     "clip_l",     "clip_t",     "clip_r",
    "clip_b",     "visited",     "visible",     "drawable",
    "truncated", "cmd_tbl_id",     "n0_addr",     "n0_flags",     "n0_kind",
    "n0_l",     "n0_t",     "n0_r",     "n0_b",
    "n0_el",     "n0_et",     "n0_er",     "n0_eb",
    "n0_tx_u",     "n0_tx_v",     "n0_tx_w",     "n0_tx_h", "n0_child", "n0_sibling", "n0_parent",
    "n1_addr",     "n1_flags",     "n1_kind",     "n1_l",
    "n1_t",     "n1_r",     "n1_b",     "n1_el",
    "n1_et",     "n1_er",     "n1_eb",     "n1_tx_u",
    "n1_tx_v",     "n1_tx_w",     "n1_tx_h", "n1_child", "n1_sibling", "n1_parent",     "n2_addr",
    "n2_flags",     "n2_kind",     "n2_l",     "n2_t",
    "n2_r",     "n2_b",     "n2_el",     "n2_et",
    "n2_er",     "n2_eb",     "n2_tx_u",     "n2_tx_v",
    "n2_tx_w",     "n2_tx_h", "n2_child", "n2_sibling", "n2_parent",     "n3_addr",     "n3_flags",
    "n3_kind",     "n3_l",     "n3_t",     "n3_r",
    "n3_b",     "n3_el",     "n3_et",     "n3_er",
    "n3_eb",     "n3_tx_u",     "n3_tx_v",     "n3_tx_w",
    "n3_tx_h", "n3_child", "n3_sibling", "n3_parent",     "n4_addr",     "n4_flags",     "n4_kind",
    "n4_l",     "n4_t",     "n4_r",     "n4_b",
    "n4_el",     "n4_et",     "n4_er",     "n4_eb",
    "n4_tx_u",     "n4_tx_v",     "n4_tx_w",     "n4_tx_h", "n4_child", "n4_sibling", "n4_parent",
    "n5_addr",     "n5_flags",     "n5_kind",     "n5_l",
    "n5_t",     "n5_r",     "n5_b",     "n5_el",
    "n5_et",     "n5_er",     "n5_eb",     "n5_tx_u",
    "n5_tx_v",     "n5_tx_w",     "n5_tx_h", "n5_child", "n5_sibling", "n5_parent",
];

/// Sample word layout: the head block, then `MAX_NODES` node blocks.
pub const HEAD_WORDS: usize = 26;
pub const NODE_WORDS: usize = 18;
/// The head block is the first 26 words of `Sample::words`, in that order. These
/// indexes are the proof: a fixture pins each one to its named slot.
pub const FIELD_VISITED: usize = 21;
pub const FIELD_VISIBLE: usize = 22;
pub const FIELD_DRAWABLE: usize = 23;

/// Word positions compared against the previous sample; a difference means the
/// state changed and the sample deserves a log line. The set covers the
/// questions Phase 1 asks: HUD recreation (controller, root), player or weapon
/// change (hunter index, player), buff or recovery (current, maximum, tier),
/// the clip the game draws inside (context, clip_*), the tree shape (counts,
/// first node address and rectangle).
pub const KEY_FIELDS: [usize; 21] = [
    0, 5, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26, 29, 30, 31, 32,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub controller: u32,
    pub row: u32,
    pub owner: u32,
    pub cache: u32,
    pub cache_flags: u32,
    pub root: u32,
    pub animation: u32,
    pub objects: u32,
    pub sequences: u32,
    pub first_object: u32,
    pub local: Local,
    pub input_flags: u32,
    pub cmd_tbl_id: u32,
    pub context: u32,
    pub clip: [i32; 4],
    pub gauge: Option<Gauge>,
    pub walk: Walk,
}

impl Sample {
    pub fn words(&self) -> [u32; FIELDS.len()] {
        let gauge = self.gauge.unwrap_or(Gauge::default());
        let mut words = [0u32; FIELDS.len()];
        let head: [u32; 26] = [
            self.controller,
            self.row,
            self.owner,
            self.cache,
            self.cache_flags,
            self.root,
            self.animation,
            self.objects,
            self.sequences,
            self.first_object,
            self.local.hunter_index,
            self.local.player,
            gauge.current as u32,
            gauge.maximum as u32,
            gauge.tier,
            self.input_flags,
            self.context,
            self.clip[0] as u32,
            self.clip[1] as u32,
            self.clip[2] as u32,
            self.clip[3] as u32,
            self.walk.visited,
            self.walk.visible,
            self.walk.drawable,
            self.walk.truncated as usize as u32,
            self.cmd_tbl_id,
        ];
        for index in 0..head.len() {
            words[index] = head[index];
        }
        for index in 0..MAX_NODES {
            let node = self.walk.nodes[index];
            let base = HEAD_WORDS + index * NODE_WORDS;
            words[base] = node.address;
            words[base + 1] = node.flags;
            words[base + 2] = node.kind;
            for edge in 0..4 {
                words[base + 3 + edge] = node.rect[edge] as u32;
                words[base + 7 + edge] = node.effective[edge] as u32;
                words[base + 11 + edge] = node.texture[edge] as u32;
            }
            words[base + 15] = node.links[0];
            words[base + 16] = node.links[1];
            words[base + 17] = node.links[2];
        }
        words
    }
}

/// Read the gauge state behind one animation controller. Returns None when the
/// controller is not the sharpness animation, so callers can ignore it cheaply.
/// `context` is the walker's draw argument, whose live clip every drawable in
/// the walk is intersected with before it is painted.
pub fn sample_controller(memory: &dyn Memory, controller: u32, context: u32, base: u32) -> Option<Sample> {
    let row = memory.word(controller.checked_add(CONTROLLER_ROW)?)?;
    if row == 0 {
        return None;
    }
    let Some(animation) = memory.word(row.checked_add(ROW_ID)?) else {
        return None;
    };
    let Some(counts) = memory.word(row.checked_add(ROW_COUNTS)?) else {
        return None;
    };
    let Some(first_object) = memory.word(row.checked_add(ROW_FIRST_OBJECT)?) else {
        return None;
    };
    // Objects and sequences are the two u16 halves of the packed count word.
    let objects = counts & 0xffff;
    let sequences = (counts >> 16) & 0xffff;
    if !is_sharp_animation(animation, objects, first_object, sequences) {
        return None;
    }
    let mut sample = Sample {
        controller,
        row,
        animation,
        objects,
        sequences,
        first_object,
        context,
        owner: memory.word(controller.checked_add(CONTROLLER_OWNER)?).unwrap_or(0),
        cache: memory.word(controller.checked_add(CONTROLLER_CACHE)?).unwrap_or(0),
        cache_flags: memory.word(controller.checked_add(CONTROLLER_FLAGS)?).unwrap_or(0),
        root: memory.word(controller.checked_add(CONTROLLER_ROOT)?).unwrap_or(0),
        ..Sample::default()
    };
    if sample.root != 0 {
        sample.walk = scan_walk(memory, sample.root);
    }
    if context != 0 {
        for edge in 0..4 {
            sample.clip[edge] = memory
                .word(context.wrapping_add(CONTEXT_CLIP + (edge as u32) * 4))
                .unwrap_or(0) as i32;
        }
    }
    if let Some(local) = local_player(memory, base) {
        sample.local = local;
        sample.gauge = read_gauge(memory, local.player);
        sample.input_flags = memory
            .word(local.player.wrapping_add(INPUT_FLAGS))
            .unwrap_or(0);
        sample.cmd_tbl_id = memory
            .word(local.player.wrapping_add(CMD_TBL_ID))
            .unwrap_or(0);
    }
    Some(sample)
}

/// Field positions of the signed gauge words inside `Sample::words`, so the
/// logger rebuilds the typed values from a copied sample without the game.
pub const FIELD_CURRENT: usize = 12;
pub const FIELD_MAXIMUM: usize = 13;
pub const FIELD_TIER: usize = 14;

/// Gauge values decoded from an already-copied sample.
pub fn gauge_of(words: &[u32]) -> Option<Gauge> {
    if words.len() < FIELDS.len() {
        return None;
    }
    Some(Gauge {
        current: words[FIELD_CURRENT] as i32,
        maximum: words[FIELD_MAXIMUM] as i32,
        tier: words[FIELD_TIER],
    })
}

/// Fill in basis points (0..=10000), or None when the state is unsupported.
pub fn ratio_of_words(words: &[u32]) -> Option<u32> {
    gauge_of(words).and_then(basis_points)
}
