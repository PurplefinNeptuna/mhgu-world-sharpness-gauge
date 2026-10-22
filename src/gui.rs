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
    hooks, memory,
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
const GAUGE_DRAWABLES: u32 = 3;
const BAR_WIDTH: f32 = 88.0;
const BAR_HEIGHT: f32 = 24.0;

/// 0 = not inserted, 1 = inserted, 2 = failed. One insertion per plugin session:
/// the row is a persistent HUD object and a second mask would clip twice.
static STATE: AtomicU8 = AtomicU8::new(0);
static MASK: AtomicU32 = AtomicU32::new(0);
static OLD_MAP: AtomicU32 = AtomicU32::new(0);
static NEW_MAP: AtomicU32 = AtomicU32::new(0);
static OLD_CACHE: AtomicU32 = AtomicU32::new(0);
static MAP_FREE: AtomicU32 = AtomicU32::new(0);
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
/// gauge animation, 25 the root's map alias disagrees, 26 root+0x50 is not 1,
/// 27 cache alias, map length or map[0] disagree, 28 controller+0x68 is not the bar,
/// 29 the bar/effect/edge chain disagrees, 30 the bar has no usable parent, 31 the
/// bar is not a cGUIObjTexture, 32 the bar's live size is not 88x24, 33 a DTI is
/// missing.
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
    let (Some(objects), Some(sequences), Some(drawables)) = (
        memory::half(description.wrapping_add(4)),
        memory::half(description.wrapping_add(6)),
        memory::word(description.wrapping_add(8)),
    ) else {
        return Err(23);
    };
    let (Some(anim), Some(map_alias), Some(active)) = (
        memory::word(description),
        memory::word(root.wrapping_add(0xa0)),
        memory::word(root.wrapping_add(0x50)),
    ) else {
        return Err(23);
    };
    if anim != GAUGE_ANIMATION
        || objects != GAUGE_OBJECTS
        || sequences != GAUGE_SEQUENCES
        || drawables != GAUGE_DRAWABLES
    {
        return Err(24);
    }
    if map_alias != map {
        return Err(25);
    }
    if active != 1 {
        return Err(26);
    }
    let (Some(cache), Some(cache_alias)) = (
        memory::word(root.wrapping_add(0xa4)),
        memory::word(root.wrapping_add(0x80)),
    ) else {
        return Err(23);
    };
    let bytes = drawables.wrapping_mul(16).wrapping_add(8) as usize;
    let (Some(head), Some(bar), Some(effect), Some(edge)) = (
        memory::word(map),
        memory::word(map.wrapping_add(4)),
        memory::word(map.wrapping_add(8)),
        memory::word(map.wrapping_add(12)),
    ) else {
        return Err(23);
    };
    if cache == 0
        || cache_alias != cache
        || !memory::readable(cache, bytes)
        || !memory::readable(map, objects as usize * 4)
        || head != root
    {
        return Err(27);
    }
    // The row's map is [controller, tree root, ...], and the gauge's tree root is
    // the bar sprite: the r5 log prints controller+0x68 equal to the first walked
    // node, which is `Txt_sharp`. Then the effect glyph, then the edge. The group
    // `sharp` is the bar's parent above the walker's entry point, derived from the
    // bar and never indexed from the map.
    let Some(tree_root) = memory::word(root.wrapping_add(0x68)) else {
        return Err(23);
    };
    if tree_root != bar {
        return Err(28);
    }
    let (Some(bar_child), Some(effect_parent), Some(effect_child), Some(edge_parent)) = (
        memory::word(bar.wrapping_add(0x68)),
        memory::word(effect.wrapping_add(0x70)),
        memory::word(effect.wrapping_add(0x68)),
        memory::word(edge.wrapping_add(0x70)),
    ) else {
        return Err(23);
    };
    if !memory::writable(bar, TEXTURE_BYTES)
        || !memory::writable(effect, TEXTURE_BYTES)
        || !memory::writable(edge, TEXTURE_BYTES)
        || bar_child != effect
        || effect_parent != bar
        || effect_child != edge
        || edge_parent != effect
    {
        return Err(29);
    }
    let Some(group) = memory::word(bar.wrapping_add(0x70)) else {
        return Err(23);
    };
    let Some(group_child) = memory::word(group.wrapping_add(0x68)) else {
        return Err(23);
    };
    if group == 0 || group == root || !memory::writable(group, NODE_BYTES) || group_child != bar {
        return Err(30);
    }
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
    // 2. A map with one more binding, keeping the game's own bindings in place.
    let instance_dti = MtDti::find("cGUIInstance").ok_or(19u32)? as *const _ as u32;
    let getter: u32 = base() + 0x7aa47c;
    let allocator = memory::call1(getter, instance_dti).ok_or(3u32)?;
    let allocate = method(allocator, 0x1c).ok_or(4u32)?;
    let free = method(allocator, 0x34).ok_or(5u32)?;
    let map_bytes = (GAUGE_OBJECTS as u32 + 1) * 4;
    let new_map = memory::call3(allocate, allocator, map_bytes, 16).ok_or(6u32)?;
    if new_map == 0 || new_map & 15 != 0 || !memory::writable(new_map, map_bytes as usize) {
        return Err(7);
    }
    // SAFETY: destination mapping checked writable, source checked readable,
    // length is the row's own binding count.
    unsafe {
        core::ptr::copy_nonoverlapping(row.map as *const u8, new_map as *mut u8, GAUGE_OBJECTS as usize * 4)
    };
    word(new_map + GAUGE_OBJECTS as u32 * 4, mask).then_some(()).ok_or(8u32)?;
    // 3. The draw cache grows by one begin/end pair: the walker emits a pair for
    // the new child, and the terminator stays last.
    let cache_bytes = (GAUGE_DRAWABLES + 1) * 16 + 8;
    let row_allocate = method(row.root, 0x58).ok_or(9u32)?;
    let new_cache = memory::call2(row_allocate, row.root, cache_bytes).ok_or(10u32)?;
    if new_cache == 0 || new_cache & 15 != 0 || !memory::write_bytes(new_cache, cache_bytes as usize) {
        memory::call2(free, allocator, new_map);
        return Err(11);
    }
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
        memory::call2(free, allocator, new_map);
        memory::call2(method(row.root, 0x5c).ok_or(14u32)?, row.root, new_cache);
        return Err(15);
    }
    // 5. Publish both map aliases and both cache aliases, then ask the row to
    // rebuild its draw order.
    let published = word(row.instance + 0xd8, new_map)
        && word(row.root + 0xa0, new_map)
        && word(row.root + 0xa4, new_cache)
        && word(row.root + 0x80, new_cache)
        && word(
            row.root + 0x8c,
            memory::word(row.root + 0x8c).ok_or(16u32)? | 3,
        );
    if !published {
        return Err(17);
    }
    // The originals stay allocated and reachable; the plugin keeps them for
    // restore, and the game's own cleanup counts the published tree.
    OLD_MAP.store(row.map, Ordering::Release);
    NEW_MAP.store(new_map, Ordering::Release);
    OLD_CACHE.store(row.cache, Ordering::Release);
    MAP_FREE.store(free, Ordering::Release);
    ALLOCATOR.store(allocator, Ordering::Release);
    SIZES.store(
        (GAUGE_OBJECTS as u32) | ((row.cache_bytes()) << 16),
        Ordering::Release,
    );
    Ok(mask)
}

impl Row {
    fn cache_bytes(&self) -> u32 {
        (GAUGE_DRAWABLES * 16 + 8) & 0xffff
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

fn restore_links(old_map: u32, old_cache: u32, instance: u32, root: u32) -> Option<()> {
    let bar = memory::word(old_map.checked_add(8)?)?;
    let group = memory::word(old_map.checked_add(4)?)?;
    if !word(bar + 0x70, group)
        || !word(group + 0x68, bar)
        || !word(instance + 0xd8, old_map)
        || !word(root + 0xa0, old_map)
        || !word(root + 0xa4, old_cache)
        || !word(root + 0x80, old_cache)
    {
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
    let Some((instance, _)) = hooks::gauge_instance() else {
        return false;
    };
    let (mask, _, _) = applied();
    let old_map = OLD_MAP.load(Ordering::Relaxed);
    let old_cache = OLD_CACHE.load(Ordering::Relaxed);
    if old_map == 0 || old_cache == 0 || restore_links(old_map, old_cache, instance, root_of(instance)).is_none() {
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
    let spare = NEW_MAP.load(Ordering::Relaxed);
    if free != 0 && allocator != 0 && spare != 0 {
        memory::call2(free, allocator, spare);
    }
    STATE.store(2, Ordering::Release);
    true
}

fn root_of(instance: u32) -> u32 {
    memory::word(instance + 0xdc).unwrap_or(0)
}
