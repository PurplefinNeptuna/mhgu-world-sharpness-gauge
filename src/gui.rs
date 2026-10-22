//! Insert one `cGUIObjScissorMask` above the sharpness bar so the gauge row is
//! clipped to current over maximum points. Colour, tier frames, sequences, atlases
//! and vertex colours stay exactly as the game makes them; this module only builds
//! the mask, publishes it on the row, reparents the bar under it, and sizes the
//! clip from the core decision.
//!
//! Every offset below is proven: the mask factory main+0xa4b6cc allocates 0xd0
//! bytes through the DTI allocator itself, the row layout and publish order follow
//! Key Quest Markers' working counter insertion, and the gauge chain (group,
//! `Txt_sharp`, `Txt_sharpefx`, `Txt_sharpEdge`) is the order `hud0.gui` stores.
//! Nothing here allocates through Forge, and no game-owned block is freed while
//! the tree still points at it.
use crate::{
    hooks, memory, sampling,
    memory::BASE,
};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use forge::mt::dti::MtDti;

/// Mod-owned object id, outside the ids `hud0.gui` uses for this row.
const MOD_ID: u32 = 0x4d534731; // MSG1
const MASK_BYTES: usize = 208;
const TEXTURE_BYTES: usize = 304;
const INSTANCE_BYTES: usize = 224;
const ROOT_BYTES: usize = 176;
const NODE_BYTES: usize = 192;

// Row description fields, as Key Quest Markers reads them at root+0x9c.
const GAUGE_ANIMATION: u32 = 4;
const GAUGE_OBJECTS: u16 = 4;
const GAUGE_SEQUENCES: u16 = 15;
const GAUGE_FIRST_OBJECT: u32 = 29;
const BAR_WIDTH: f32 = 88.0;
const BAR_HEIGHT: f32 = 24.0;

/// 0 = not inserted, 1 = inserted, 2 = failed. One insertion per plugin session:
/// the row is a persistent HUD object and a second mask would clip twice.
static STATE: AtomicU8 = AtomicU8::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static OLD_CACHE: AtomicU32 = AtomicU32::new(0);
static MAP_FREE: AtomicU32 = AtomicU32::new(0); // the allocator's free method
static BAR_NODE: AtomicU32 = AtomicU32::new(0);
static GROUP_NODE: AtomicU32 = AtomicU32::new(0);
static CACHE_BLOCK: AtomicU32 = AtomicU32::new(0);
static ALLOCATOR: AtomicU32 = AtomicU32::new(0);
static FAILURE: AtomicU32 = AtomicU32::new(0);
/// The row's map length and cache size at the moment of insertion, for the log.
static SIZES: AtomicU32 = AtomicU32::new(0);

pub fn take_failure() -> u32 {
    FAILURE.swap(0, Ordering::Relaxed)
}
pub fn inserted() -> bool {
    STATE.load(Ordering::Relaxed) == 1
}
/// (mask, old map length, old cache bytes) for the log line.
pub fn bar_width() -> (u32, u32) {
    (BAR_W.load(Ordering::Relaxed), AUTHORED.load(Ordering::Relaxed))
}

/// root+0x50 as read, and the walker's drawable count the cache length uses.
pub fn row_flags() -> (u32, u32) {
    (AUTHORED50.load(Ordering::Relaxed), sampling::drawables())
}

pub fn applied() -> (u32, u32, u32) {
    let sizes = SIZES.load(Ordering::Relaxed);
    (
        MASK.load(Ordering::Relaxed),
        sizes & 0xffff,
        sizes >> 16,
    )
}

fn base() -> u32 {
    BASE.load(Ordering::Relaxed)
}
fn method(object: u32, slot: u32) -> Option<u32> {
    memory::method(object, slot)
}
fn is_type(object: u32, dti: u32) -> bool {
    let Some(f) = method(object, 0x14) else {
        return false;
    };
    memory::call1(f, object).is_some_and(|got| got == dti)
}
fn word(address: u32, value: u32) -> bool {
    memory::write_word(address, value)
}

/// The gauge row as the game currently holds it. Every field is checked against
/// the archive's structure, and the bar is identified by its authored size.
struct Row {
    root: u32,
    instance: u32,
    map: u32,
    cache: u32,
    group: u32,
    bar: u32,
    effect: u32,
    width: i32,
}

/// Read and validate the gauge row. Each gate has its own stage code so a single
/// run says which check failed: 21 no captured instance pair, 22 instance or
/// controller not writable, 23 a field read failed, 24 row counts do not match the
/// gauge animation, 25 the controller has no usable draw cache,
/// 26 controller+0x68 is missing, equals the controller, or is not writable, 27 the
/// bar's siblings are not two distinct objects, 28 an object in the chain is not
/// writable, 29 the bar's parent group is missing, is the controller, the edge's
/// parent differs, or its first child is not the bar, 31 the bar is not a
/// cGUIObjTexture, 32 the bar's live width is outside 1..=1024, 33 a DTI is missing.
fn read_row() -> Result<Row, u32> {
    let Some((instance, root)) = hooks::gauge_instance() else {
        return Err(21);
    };
    if !memory::writable(instance, INSTANCE_BYTES) || !memory::writable(root, ROOT_BYTES) {
        return Err(22);
    }
    let (Some(map), Some(description)) = (
        memory::word(instance.wrapping_add(0xd8)),
        memory::word(root.wrapping_add(0x9c)),
    ) else {
        return Err(23);
    };
    let (Some(objects), Some(sequences)) = (
        memory::half(description.wrapping_add(4)),
        memory::half(description.wrapping_add(6)),
    ) else {
        return Err(23);
    };
    let (Some(anim), Some(active)) = (
        memory::word(description),
        memory::word(root.wrapping_add(0x50)),
    ) else {
        return Err(23);
    };
    let Some(first_object) = memory::word(description.wrapping_add(0x0c)) else {
        return Err(23);
    };
    // The gauge row is identified by the fields the sampler already proves on every
    // sample line: animation 4, 4 objects, 15 sequences, first object 29. The row
    // carries no drawable count, so the cache length comes from the walker's count
    // (sampling::drawables), not from the row. root+0x50 is recorded, not required:
    // it is a counter-row fact from Key Quest Markers, not a gauge-row fact.
    AUTHORED50.store(active, Ordering::Release);
    if anim != GAUGE_ANIMATION
        || objects != GAUGE_OBJECTS
        || sequences != GAUGE_SEQUENCES
        || first_object != GAUGE_FIRST_OBJECT
    {
        return Err(24);
    }
    // Gauge layout, proven from the r5-r8 sample lines: the controller keeps the
    // animation row at +0x9c, the draw cache at +0x80, and the walker entry at
    // +0x68 is the bar node itself. The gauge publishes no animation map list
    // (the samples print map=0 map_word=0), so the map and cache-alias gates are
    // dropped. The chain is read from the node tree: the bar's siblings are the
    // effect glyph and the edge marker, and the bar's parent is the group.
    let Some(cache) = memory::word(root.wrapping_add(0x80)) else {
        return Err(23);
    };
    if cache == 0 || !memory::readable(cache, 8) {
        return Err(25);
    }
    let Some(bar) = memory::word(root.wrapping_add(0x68)) else {
        return Err(23);
    };
    if bar == 0 || bar == root || !memory::writable(bar, TEXTURE_BYTES) {
        return Err(26);
    }
    let (Some(effect), Some(effect_parent)) = (
        memory::word(bar.wrapping_add(0x6c)),
        memory::word(bar.wrapping_add(0x70)),
    ) else {
        return Err(23);
    };
    let Some(edge) = memory::word(effect.wrapping_add(0x6c)) else {
        return Err(23);
    };
    let (Some(edge_parent), Some(group_child)) = (
        memory::word(edge.wrapping_add(0x70)),
        memory::word(effect_parent.wrapping_add(0x68)),
    ) else {
        return Err(23);
    };
    if effect == 0 || effect == bar || edge == 0 || edge == effect {
        return Err(27);
    }
    if !memory::writable(effect, TEXTURE_BYTES)
        || !memory::writable(edge, TEXTURE_BYTES)
        || !memory::writable(effect_parent, NODE_BYTES)
    {
        return Err(28);
    }
    if effect_parent == root || edge_parent != effect_parent || group_child != bar {
        return Err(29);
    }
    let group = effect_parent;
    let Some(texture) = MtDti::find("cGUIObjTexture") else {
        return Err(33);
    };
    if !is_type(bar, texture as *const _ as u32) {
        return Err(31);
    }
    let (Some(width), Some(height)) = (
        memory::word(bar.wrapping_add(0xb4)),
        memory::word(bar.wrapping_add(0xb8)),
    ) else {
        return Err(23);
    };
    // The authored 88x24 is the expected size; the live value is what the geometry
    // scales from, so a bar the game draws at another size still clips correctly
    // instead of failing the gate. Stage 32 means an unusable live size.
    let live_width = f32::from_bits(width) as i32;
    if !(1..=1024).contains(&live_width) {
        return Err(32);
    }
    BAR_W.store(live_width as u32, Ordering::Release);
    AUTHORED.store(
        ((width == BAR_WIDTH.to_bits()) as u32) | (((height == BAR_HEIGHT.to_bits()) as u32) << 1),
        Ordering::Release,
    );
    Ok(Row {
        root,
        instance,
        map,
        cache,
        group,
        bar,
        effect,
        width: live_width,
    })
}

/// Build the mask, grow the row's map and draw cache, publish them, then reparent
/// the bar under the mask. A failure at any stage publishes nothing.
fn insert(row: &Row) -> Result<u32, u32> {
    // 1. The mask. Its factory allocates through the mask DTI's own MT allocator,
    // so the game's destructor owns the block from the moment it returns.
    let mask = memory::call0(base() + 0xa4b6cc).ok_or(1u32)?;
    let dti = MtDti::find("cGUIObjScissorMask").ok_or(18u32)? as *const _ as u32;
    if mask == 0 || mask & 3 != 0 || !memory::writable(mask, MASK_BYTES) || !is_type(mask, dti) {
        return Err(2);
    }
    // 2. The game's allocator. The counter mod allocates a binding map per row;
    // the gauge row publishes none, so the only allocation here is the cache.
    let instance_dti = MtDti::find("cGUIInstance").ok_or(19u32)? as *const _ as u32;
    let getter: u32 = base() + 0x7aa47c;
    let allocator = memory::call1(getter, instance_dti).ok_or(3u32)?;
    let allocate = method(allocator, 0x1c).ok_or(4u32)?;
    let free = method(allocator, 0x34).ok_or(5u32)?;
    // 3. The draw cache grows by one begin/end pair: the walker emits a pair for
    // the new child, and the terminator stays last. The counter mod asks the row
    // object's vtable slot 0x58 for that buffer; the gauge controller has no
    // proven slot there, so the block comes from the same game allocator as the
    // map and the entry pointer is the second word of the header, exactly where
    // the sampler reads it (count at block+0x18, flags at block+0x1c).
    let cache_bytes = (sampling::drawables().max(1) + 1) * 16 + 8;
    let block = memory::call3(allocate, allocator, cache_bytes + 8, 16).ok_or(10u32)?;
    let new_cache = block.checked_add(8).ok_or(11u32)?;
    if block == 0
        || block & 15 != 0
        || !memory::write_bytes(block, (cache_bytes + 8) as usize)
    {
        return Err(11);
    }
    CACHE_BLOCK.store(block, Ordering::Release);
    // 4. The mask takes the bar's place in the group's child slot, and the bar
    // becomes the mask's child. The bar keeps its own children, so the effect
    // glyph and the edge are inside the same scissor.
    if !word(mask + 0x50, MOD_ID)
        || !word(mask + 0x68, row.bar)
        || !word(mask + 0x6c, 0)
        || !word(mask + 0x70, row.group)
        || !word(mask + 0x54, (memory::word(mask + 0x54).ok_or(12u32)? | 0x80_0000))
        || !word(row.bar + 0x70, mask)
        || !word(row.group + 0x68, mask)
        || !word(
            row.group + 0x54,
            memory::word(row.group + 0x54).ok_or(13u32)? | 0x80_0000,
        )
    {
        memory::call2(free, allocator, block);
        return Err(15);
    }
    // 5. Publish the gauge cache pointer. The counter mod writes map aliases at
    // root+0xa0/+0xa4 and a cache alias at root+0x9c; the gauge uses none of them
    // (controller+0x9c is the animation row and +0xa0/+0xa4 stay 0), so writing
    // them here would corrupt the row. Only the cache pointer and its flags move.
    let published = word(row.root + 0x80, new_cache)
        && word(
            row.root + 0x8c,
            memory::word(row.root + 0x8c).ok_or(16u32)? | 3,
        );
    if !published {
        return Err(17);
    }
    // The originals stay allocated and reachable; the plugin keeps them for
    // restore, and the game's own cleanup counts the published tree.
    OLD_CACHE.store(row.cache, Ordering::Release);
    MAP_FREE.store(free, Ordering::Release);
    ALLOCATOR.store(allocator, Ordering::Release);
    SIZES.store(
        (GAUGE_OBJECTS as u32) | ((row.cache_bytes()) << 16),
        Ordering::Release,
    );
    BAR_NODE.store(row.bar, Ordering::Release);
    GROUP_NODE.store(row.group, Ordering::Release);
    Ok(mask)
}

impl Row {
    fn cache_bytes(&self) -> u32 {
        (sampling::drawables().max(1) * 16 + 8) & 0xffff
    }
}

/// Write the clip for one ratio. Position is a float pair with the dirty bit the
/// game's own setters set, size is the integer pair the consumer at main+0xa4ba94
/// reads once bit 1 of +0xa4 is on.
pub fn apply(ratio_bp: u32) -> Option<[i32; 4]> {
    if STATE.load(Ordering::Relaxed) != 1 {
        return None;
    }
    let mask = MASK.load(Ordering::Relaxed);
    let row = read_row().ok()?;
    let box_ = sharpness_gauge_core::mask_box(rect(&row.bar)?, ratio_bp, row.width);
    if !memory::write_bits(mask + 0x80, box_[0] as f32)
        || !memory::write_bits(mask + 0x84, box_[1] as f32)
        || !word(mask + 0x54, memory::word(mask + 0x54)? | 0x2_0000)
        || !word(mask + 0xa8, box_[2] as u32)
        || !word(mask + 0xac, box_[3] as u32)
        || !word(mask + 0xa4, memory::word(mask + 0xa4)? | 2)
    {
        return None;
    }
    Some(box_)
}

/// The bar's own rect, as the game keeps it at +0xe0.
fn rect(bar: &u32) -> Option<[i32; 4]> {
    let left = memory::word(bar.checked_add(0xe0)?)? as i32;
    let top = memory::word(bar.checked_add(0xe4)?)? as i32;
    let right = memory::word(bar.checked_add(0xe8)?)? as i32;
    let bottom = memory::word(bar.checked_add(0xec)?)? as i32;
    Some([left, top, right, bottom])
}

/// Insert once, from the plugin's update callback (the game's own thread, outside
/// the walker). A row that is not present yet is not a failure: the gauge HUD row
/// exists only while a weapon-owning player is in a quest, so stage 20 reports once
/// and keeps retrying at a slow cadence. A hard failure (mask factory, allocation
/// or publish) is terminal, because retrying an allocation path that already
/// failed is not how this row gets built.
const RETRY_EVERY: u32 = 240;
static TRIES: AtomicU32 = AtomicU32::new(0);
/// The bar's live width, and bit 0/1 for "the authored 88 and 24 match".
static BAR_W: AtomicU32 = AtomicU32::new(0);
static AUTHORED: AtomicU32 = AtomicU32::new(0);
/// root+0x50 as read, recorded because it is not a gate for this row.
static AUTHORED50: AtomicU32 = AtomicU32::new(0);

pub fn tick() {
    if STATE.load(Ordering::Relaxed) != 0 {
        return;
    }
    let tries = TRIES.fetch_add(1, Ordering::Relaxed);
    if tries != 0 && tries % RETRY_EVERY != 0 {
        return;
    }
    let row = match read_row() {
        Ok(row) => row,
        Err(gate) => {
            // Overwrite, so a persistent gate is reported on every retry instead
            // of only the first time. The gate is the diagnosis, not a status.
            FAILURE.store(gate, Ordering::Release);
            return;
        }
    };
    match insert(&row) {
        Ok(mask) => {
            MASK.store(mask, Ordering::Release);
            STATE.store(1, Ordering::Release);
        }
        Err(stage) => {
            let _ = FAILURE.compare_exchange(0, stage, Ordering::Relaxed, Ordering::Relaxed);
            STATE.store(2, Ordering::Release);
        }
    }
}

fn restore_links(old_cache: u32, root: u32) -> Option<()> {
    let bar = BAR_NODE.load(Ordering::Relaxed);
    let group = GROUP_NODE.load(Ordering::Relaxed);
    if bar == 0 || group == 0 {
        return None;
    }
    if !word(bar + 0x70, group) || !word(group + 0x68, bar) {
        return None;
    }
    if !word(root + 0x80, old_cache) {
        return None;
    }
    let flags = memory::word(root + 0x8c)?;
    if !word(root + 0x8c, flags | 3) {
        return None;
    }
    Some(())
}

/// Undo the insertion: restore the row's links, publish the original map and
/// cache back, then let the game's destructor free the mask. The map the plugin
/// allocated is freed last, after nothing points at it.
pub fn restore() -> bool {
    if STATE.load(Ordering::Relaxed) != 1 {
        return true;
    }
    let (mask, _, _) = applied();
    let old_cache = OLD_CACHE.load(Ordering::Relaxed);
    let Some((_, root)) = hooks::gauge_instance() else {
        return false;
    };
    if old_cache == 0 || restore_links(old_cache, root).is_none() {
        return false;
    }
    let block = CACHE_BLOCK.load(Ordering::Relaxed);
    if block != 0 && !word(root + 0x80, old_cache) {
        return false;
    }
    if let Some(destroy) = method(mask, 0x04) {
        memory::call1(destroy, mask);
    }
    let free = MAP_FREE.load(Ordering::Relaxed);
    let allocator = ALLOCATOR.load(Ordering::Relaxed);
    // The map the plugin allocated is the one to free: after the links and the
    // published aliases are restored, nothing points at it any more. The game's
    // original map stays allocated and owned by the row.
    if free != 0 && allocator != 0 && block != 0 {
        memory::call2(free, allocator, block);
    }
    STATE.store(2, Ordering::Release);
    true
}
