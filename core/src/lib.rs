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
// counts (objects low, sequences high); +12 is the first object row index; +20
// is a pointer to the first sequence block, not a row index, which is why it is
// not part of the identity. The gauge identity tuple is identical in every
// inspected language archive and in the HD copy, so identification never uses
// names, strings or a language prefix.
/// A full gauge, in the basis points `basis_points` returns: the tip's screen position is
/// the cut box's width here, and the hidden blade is everything past it.
pub const FULL_SHARPNESS_BP: u32 = 10_000;

pub const SHARP_ANIMATION: u32 = 4;
pub const SHARP_OBJECTS: u32 = 4;
pub const SHARP_FIRST_OBJECT: u32 = 29;
pub const SHARP_SEQUENCES: u32 = 15;
pub const ROW_ID: u32 = 0x00;
pub const ROW_COUNTS: u32 = 0x04;
pub const ROW_FIRST_OBJECT: u32 = 0x0c;

// Animation controller: +0x9c animation row is what identification reads. The tree root at
// +0x68, the draw cache at +0x80, the rebuild flags at +0x8c and the owner at +0xa0 are the
// fields the insertion in `src/insert.rs` reads by their own offsets.
pub const CONTROLLER_ROW: u32 = 0x9c;

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
/// glow. It is not the draw width: the bar's transform x scale is 1.0, so the sprite fills the
/// node's screen rect at `+0xE0`, and that rect is what the fill's geometry is measured on.
pub const NODE_AUTHORED_WIDTH: u32 = 0xb4;
pub const NODE_RECT: u32 = 0xe0;
pub const NODE_EFFECTIVE_RECT: u32 = 0xf0;
pub const NODE_TEXTURE_RECT: u32 = 0x10c;
pub const NODE_CHILD: u32 = 0x68;
pub const NODE_SIBLING: u32 = 0x6c;
pub const NODE_PARENT: u32 = 0x70;
pub const NODE_VISIBLE: u32 = 1;
pub const NODE_DRAWABLE: u32 = 0x0200_0000;

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

/// The fill as integer basis points, so the runtime never formats a float:
/// `current * 10_000 / maximum` clamped to `0..=10_000`. A positive maximum is
/// required before any division; depletion below zero draws empty and an
/// above-maximum value draws full. A zero, negative or unreadable maximum is
/// unsupported state and yields None, which leaves the native gauge alone.
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

/// The bar's rows inside the clip. `Txt_sharp` draws its ink in rows +6..=+17 of its 88x24
/// glyph frames (hud00 rows 280/304/328) and the effect glyph's authored offset is (6,-6) with
/// height 32, so the box starts `CLIP_TOP` above the bar rect's top and is `CLIP_HEIGHT` tall:
/// the bar's ink rows and the effect glyph's offset rows both fall inside. Measured 2026-10-22
/// from `HD_hud00_BM_NOMIP.tex` (sha256
/// bd5d1a3fcc77454ef8a7ed715dda1a76a72e5e0ee6413d4ddc6f550c7b89e857).
pub const CLIP_TOP: i32 = -6;
pub const CLIP_HEIGHT: i32 = 32;

/// The accepted cut endpoints as pixels from the bar rect's left, measured on that rect's own
/// 128 px width: 18 at 0% and 90 at 100%, i.e. screen 128 and 200 px on the 110..238 bar.
const CUT_START: i32 = 18;
const CUT_END: i32 = 90;
const CUT_MEASURED_WIDTH: i32 = 128;

/// The scissor box for one sharpness ratio. The kept part grows from the rect's left, which is
/// the hilt, and the tip side is clipped; the effect glyph is clipped at the same edge because
/// its rect carries its `+6,-6` local offset. The accepted endpoints are offsets on the measured
/// rect, so they scale with the live rect width and a bar drawn at another scale cuts at the
/// same fraction of its blade.
pub fn cut_box(rect: [i32; 4], ratio_bp: u32) -> [i32; 4] {
    let measured = (rect[2] - rect[0]).max(1);
    span_box(
        rect,
        ratio_bp,
        measured * CUT_START / CUT_MEASURED_WIDTH,
        measured * CUT_END / CUT_MEASURED_WIDTH,
    )
}

/// The box from two endpoints measured in pixels from the bar rect's left: the box's right edge
/// is at `start` at ratio 0 and at `end` at ratio 10000, linear between them. Width is never
/// below 1 and never past the rect's right, so the degenerate zero-size scissor is never asked
/// for.
pub fn span_box(rect: [i32; 4], ratio_bp: u32, start: i32, end: i32) -> [i32; 4] {
    let width = (start + (end - start) * ratio_bp as i32 / 10_000).max(1);
    [
        rect[0],
        rect[1] + CLIP_TOP,
        width.min(rect[2] - rect[0]),
        CLIP_HEIGHT,
    ]
}

/// The shade's quad for this cut, in the bar's own pixels and texture columns: the columns of its
/// blade past the cut, drawn at the bar's own scale. Returns `(kept, columns_past, box_left,
/// box_right)` with `kept` the accepted cut's width, `columns_past = the bar's column count - kept`
/// (0 at full sharpness, the blade's full width at 0%), the box from the cut's edge to the bar
/// rect's right edge.
///
/// A texture node's draw maps its atlas window into the rectangle its draw is clipped to. Giving the
/// copy the bar's whole authored quad inside that box draws the blade shrunk into the shade box - the
/// r57 photos: the whole blade squeezed in, and 1 px of nothing at full sharpness. Giving it the
/// window, the authored pair and the position of the columns past the cut draws those columns 1:1.
pub fn shade_quad(rect: [i32; 4], columns: i32, ratio_bp: u32) -> (i32, i32, i32, i32) {
    let kept = cut_box(rect, ratio_bp)[2].min(columns.max(0));
    (kept, (columns - kept).max(0), rect[0] + kept, rect[2])
}

/// The rectangle a scissor mask builds for itself when its enter runs with the rebuild bit set:
/// main+0xa4ba94 turns the authored pair at `+0xa8/+0xac` into floats, hands it with the node to
/// main+0xa4b390, and folds the four resulting corners into `+0xb0..+0xbc` (min, min, max, max).
/// `matrix` is the node's 16 words at `+0x10..+0x4c` row-major, `authored` its pair. The corner is
/// the node's own matrix translation at `+0x40/+0x44`: the builder reads no parent field and no
/// rectangle at `+0xe0`, and a mask object is 0xd0 bytes, so `+0xe0` is past its end.
///
/// shortcut: only the anchor modes this mod writes (bits 0-3 of `+0xa0` cleared, both axes 0,
/// spanning `0..size`) are modelled, and modes 1/2/3 are not. The accepted mask's box
/// `110 38 110+kept 166` from authored `(kept, 128)` pins the template's rows this folds over.
pub fn mask_box(matrix: [f32; 16], authored: [i32; 2]) -> [i32; 4] {
    let (x_row_x, y_row_y, x_row_y, y_row_x, z_row_x, z_row_y) =
        (matrix[0], matrix[5], matrix[1], matrix[4], matrix[8], matrix[9]);
    let (width, height) = (authored[0] as f32, authored[1] as f32);
    // The x-end and y-end terms main+0xa4b390 names `fVar11` and `fVar10`, per corner.
    let x_span = x_row_x * width + y_row_x * x_row_x * height;
    let y_span = y_row_y * x_row_x * height + x_row_y * width;
    [
        (matrix[12] + z_row_x) as i32,
        (matrix[13] + z_row_y) as i32,
        (matrix[12] + z_row_x + x_span) as i32,
        (matrix[13] + z_row_y + y_span) as i32,
    ]
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

/// The draw-cache sizing of main+0xa49b28: `size = (objects << 4 | 8) + sequences * 12`,
/// allocated through the root's vtable slot 0x58 and stored at `root+0xa4`; the pointer the
/// walker draws from, `root+0x80`, is `alloc + sequences * 12`, and it is written only when
/// objects is nonzero. The record area is `(objects << 4) | 8` bytes: 8 bytes per record, a
/// drawable's enter and exit pair takes 16, and the terminator takes 8. The gauge row is sized
/// for 3 objects and 16 sequences (a 56-byte area, 7 records) and the tree with an inserted mask
/// emits 9 records, so the insertion rebuilds the block for 4 objects (72 bytes): see
/// `grow_cache` in `src/insert.rs`. The row's other pair, at row+4, is 4 objects and 15
/// sequences; the sizer does not read it. Returns (size, prefix, record_area).
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

/// One bounded observation copied out inside the hot hook: the gauge row's identity and the
/// local player. All values are plain u32 words (signed fields are two's complement), so the hook
/// performs no allocation, formatting or file access. `FIELDS` names each word of `words` in the
/// same order, and `FIELD_*` are the gauge trio's slots in it.
pub const FIELDS: [&str; 11] = [
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
    "tier",
];
pub const FIELD_CURRENT: usize = 8;
pub const FIELD_MAXIMUM: usize = 9;
pub const FIELD_TIER: usize = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub controller: u32,
    pub row: u32,
    pub animation: u32,
    pub objects: u32,
    pub sequences: u32,
    pub first_object: u32,
    pub local: Local,
    /// The local player's sharpness triple, when the session chain reached a player.
    pub gauge: Option<Gauge>,
}

impl Sample {
    pub fn words(&self) -> [u32; FIELDS.len()] {
        let gauge = self.gauge.unwrap_or(Gauge::default());
        [
            self.controller,
            self.row,
            self.animation,
            self.objects,
            self.sequences,
            self.first_object,
            self.local.hunter_index,
            self.local.player,
            gauge.current as u32,
            gauge.maximum as u32,
            gauge.tier,
        ]
    }
}

/// Identify one animation controller as the sharpness gauge row and copy its identity with the
/// local player: `base` is the loaded main module, whose session and `sPlayer` cells give the
/// player pointer. Returns None for every other controller, which is the cheap filter in front of
/// the walker's hot path.
pub fn sample_controller(memory: &dyn Memory, controller: u32, base: u32) -> Option<Sample> {
    let row = memory.word(controller.checked_add(CONTROLLER_ROW)?)?;
    if row == 0 {
        return None;
    }
    let animation = memory.word(row.checked_add(ROW_ID)?)?;
    let counts = memory.word(row.checked_add(ROW_COUNTS)?)?;
    let first_object = memory.word(row.checked_add(ROW_FIRST_OBJECT)?)?;
    // Objects and sequences are the two u16 halves of the packed count word.
    let objects = counts & 0xffff;
    let sequences = (counts >> 16) & 0xffff;
    if !is_sharp_animation(animation, objects, first_object, sequences) {
        return None;
    }
    let local = local_player(memory, base).unwrap_or_default();
    let gauge = (local.player != 0).then(|| read_gauge(memory, local.player)).flatten();
    Some(Sample {
        controller,
        row,
        animation,
        objects,
        sequences,
        first_object,
        local,
        gauge,
    })
}
