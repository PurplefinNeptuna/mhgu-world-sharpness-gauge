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

// Animation row (nGUI::ANIM): +0 id u32, +4/+6/+8 u16 objects/sequences/
// drawables, +12 first object row, +20 first sequence row. The gauge identity
// tuple is identical in every inspected language archive and in the HD copy,
// so identification never uses names, strings or a language prefix.
pub const SHARP_ANIMATION: u32 = 4;
pub const SHARP_OBJECTS: u32 = 4;
pub const SHARP_FIRST_OBJECT: u32 = 29;
pub const SHARP_SEQUENCES: u32 = 15;
pub const ROW_ID: u32 = 0x00;
pub const ROW_OBJECTS: u32 = 0x04;
pub const ROW_SEQUENCES: u32 = 0x06;
pub const ROW_FIRST_OBJECT: u32 = 0x0c;
pub const ROW_FIRST_SEQUENCE: u32 = 0x14;

// Animation controller: +0x68 tree root node, +0x80 draw cache, +0x8c rebuild
// flags (bit 0 asks the walker to rebuild), +0x9c animation row, +0xa0 owner.
pub const CONTROLLER_ROOT: u32 = 0x68;
pub const CONTROLLER_CACHE: u32 = 0x80;
pub const CONTROLLER_FLAGS: u32 = 0x8c;
pub const CONTROLLER_ROW: u32 = 0x9c;
pub const CONTROLLER_OWNER: u32 = 0xa0;

// GUI object node, as walked by the draw walker at main+0xa494a0: +0x54 flags
// (bit 0 visible, 0x2000_0000 drawable), +0x38..+0x44 the four integer screen
// rectangle edges, +0x68 first child, +0x6c next sibling, +0x70 parent.
pub const NODE_FLAGS: u32 = 0x54;
pub const NODE_RECT: u32 = 0x38;
pub const NODE_CHILD: u32 = 0x68;
pub const NODE_SIBLING: u32 = 0x6c;
pub const NODE_PARENT: u32 = 0x70;
pub const NODE_VISIBLE: u32 = 1;
pub const NODE_DRAWABLE: u32 = 0x2000_0000;

/// Local hunter's player object, or None when the quest/session state does not
/// provide one. Every pointer in the chain is validated by the Memory impl.
/// `base` is the loaded main module address: the two cells below are
/// main-relative, the player pointers they yield are absolute heap addresses.
pub fn local_player(memory: &dyn Memory, base: u32) -> Option<u32> {
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
    (player != 0).then_some(player)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gauge {
    /// mSharpness, signed sharpness points.
    pub current: i32,
    /// mMaxSharpness, signed sharpness points.
    pub maximum: i32,
    /// mSharpnessType, native tier index (0 red .. 6 purple).
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

/// Bounded pre-order scan of a controller's node tree. It records counts and
/// the first drawable node only; no pointer is retained past the call. The
/// stack depth and visit count are fixed, so a damaged tree cannot run away.
pub const SCAN_STACK: usize = 16;
pub const SCAN_LIMIT: u32 = 64;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeScan {
    pub visited: u32,
    pub visible: u32,
    pub drawable: u32,
    /// First drawable node found, with its flags and screen rectangle.
    pub node: u32,
    pub flags: u32,
    pub rect: [i32; 4],
    pub truncated: bool,
}

pub fn scan_tree(memory: &dyn Memory, root: u32) -> TreeScan {
    let mut out = TreeScan::default();
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
        out.visited += 1;
        if flags & NODE_VISIBLE != 0 {
            out.visible += 1;
        }
        if flags & NODE_DRAWABLE != 0 && out.node == 0 {
            out.node = node;
            out.flags = flags;
            for index in 0..4u32 {
                out.rect[index as usize] = memory
                    .word(node.wrapping_add(NODE_RECT + index * 4))
                    .unwrap_or(0) as i32;
            }
        }
        if flags & NODE_DRAWABLE != 0 {
            out.drawable += 1;
        }
        // Push the sibling first so the child branch is visited next, which
        // keeps the recorded drawable near the top of the gauge subtree.
        push!(memory
            .word(node.wrapping_add(NODE_SIBLING))
            .unwrap_or(0));
        push!(memory.word(node.wrapping_add(NODE_CHILD)).unwrap_or(0));
    }
    out.truncated = overflow || top > 0 || out.visited >= SCAN_LIMIT;
    out
}

/// One bounded observation copied out inside the hot hook. All values are
/// plain u32 words (signed fields are two's complement) so the hook performs
/// no allocation, formatting or file access. `FIELDS` names each word in the
/// same order as `words`, which is the log format and the ring-buffer layout.
pub const FIELDS: [&str; 24] = [
    "ctrl",
    "row",
    "anim",
    "objects",
    "sequences",
    "first_object",
    "first_sequence",
    "owner",
    "cache",
    "cache_flags",
    "root",
    "visited",
    "visible",
    "drawable",
    "node",
    "node_flags",
    "rect_l",
    "rect_t",
    "rect_r",
    "rect_b",
    "player",
    "current",
    "maximum",
    "tier",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub controller: u32,
    pub row: u32,
    pub animation: u32,
    pub objects: u32,
    pub sequences: u32,
    pub first_object: u32,
    pub first_sequence: u32,
    pub owner: u32,
    pub cache: u32,
    pub cache_flags: u32,
    pub root: u32,
    pub scan: TreeScan,
    pub player: u32,
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
            self.first_sequence,
            self.owner,
            self.cache,
            self.cache_flags,
            self.root,
            self.scan.visited,
            self.scan.visible,
            self.scan.drawable,
            self.scan.node,
            self.scan.flags,
            self.scan.rect[0] as u32,
            self.scan.rect[1] as u32,
            self.scan.rect[2] as u32,
            self.scan.rect[3] as u32,
            self.player,
            gauge.current as u32,
            gauge.maximum as u32,
            gauge.tier,
        ]
    }
}

/// Read the gauge state behind one animation controller. Returns None when the
/// controller is not the sharpness animation, so callers can ignore it cheaply.
pub fn sample_controller(memory: &dyn Memory, controller: u32, base: u32) -> Option<Sample> {
    let row = memory.word(controller.checked_add(CONTROLLER_ROW)?)?;
    if row == 0 {
        return None;
    }
    let word = |offset: u32| memory.word(row.checked_add(offset)?);
    let (Some(animation), Some(counts), Some(first_object), Some(first_sequence)) = (
        word(ROW_ID),
        word(ROW_OBJECTS),
        word(ROW_FIRST_OBJECT),
        word(ROW_FIRST_SEQUENCE),
    ) else {
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
        first_sequence,
        owner: memory.word(controller.checked_add(CONTROLLER_OWNER)?)?,
        cache: memory.word(controller.checked_add(CONTROLLER_CACHE)?)?,
        cache_flags: memory.word(controller.checked_add(CONTROLLER_FLAGS)?)?,
        root: memory.word(controller.checked_add(CONTROLLER_ROOT)?)?,
        ..Sample::default()
    };
    if sample.root != 0 {
        sample.scan = scan_tree(memory, sample.root);
    }
    sample.player = local_player(memory, base).unwrap_or(0);
    if sample.player != 0 {
        sample.gauge = read_gauge(memory, sample.player);
    }
    Some(sample)
}

/// Field positions of the signed gauge words inside `Sample::words`, so the
/// logger rebuilds the typed values from a copied sample without the game.
pub const FIELD_CURRENT: usize = 21;
pub const FIELD_MAXIMUM: usize = 22;
pub const FIELD_TIER: usize = 23;

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
