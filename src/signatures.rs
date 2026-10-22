//! Exact 32-byte windows from the original main NSO. Ghidra's imported .text differs
//! from the target, so signatures come only from these bytes (Phase 0 records in
//! work/modern-sharpness-gauge/PHASE0_RESEARCH.md). scripts/verify.py rechecks every
//! window against the supplied input before a candidate is packaged.
pub const HOOK: u32 = 0xa494a0;
/// The instance draw entry main+0xa39300: the caller of the walker. Its 12-word
/// window is already a required signature below, and its first argument is the
/// cGUIInstance that owns the row's controller at instance+0xdc.
/// Same 12-word pattern as the SIGNATURES entry for 0xa39300, from original main.
pub const INSTANCE_HOOK: u32 = 0xa39300;
pub const SIGNATURES: &[(u32, [u8; 32])] = &[
    // main+0xa494a0: draw walker entry; the hook target. push {r4,r5,r11,lr} prologue, unique 12-word pattern
    (0xa494a0, [
        48, 72, 45, 233, 1, 64, 160, 225,
        128, 16, 144, 229, 0, 0, 81, 227,
        48, 136, 189, 8, 140, 32, 208, 229,
        1, 0, 18, 227, 64, 0, 0, 10,
    ]),
    // main+0xa49b28: animation controller setup; sizes the draw cache from the animation row counts
    (0xa49b28, [
        16, 64, 45, 233, 0, 64, 160, 225,
        0, 48, 160, 227, 156, 16, 132, 229,
        160, 32, 132, 229, 8, 0, 145, 229,
        32, 24, 160, 225, 32, 8, 83, 225,
    ]),
    // main+0xa39300: instance draw; the caller that hands the controller to the draw walker
    (0xa39300, [
        240, 79, 45, 233, 28, 176, 141, 226,
        4, 208, 77, 226, 6, 139, 45, 237,
        128, 208, 77, 226, 31, 208, 195, 231,
        1, 64, 160, 225, 0, 80, 160, 225,
    ]),
    // main+0xa37c1c: scissor routine; writes the effective clip into the draw object
    (0xa37c1c, [
        240, 65, 45, 233, 1, 10, 145, 237,
        49, 42, 144, 237, 4, 32, 144, 229,
        51, 26, 144, 237, 3, 58, 145, 237,
        0, 112, 160, 227, 200, 224, 130, 226,
    ]),
    // main+0xa4e628: sprite prepaint; intersects the live scissor rect with the sprite rect
    (0xa4e628, [
        240, 72, 45, 233, 16, 208, 77, 226,
        0, 80, 160, 225, 1, 64, 160, 225,
        5, 96, 160, 225, 204, 0, 182, 229,
        4, 16, 150, 229, 1, 0, 17, 227,
    ]),
    // main+0x61912c: HUD widget driver; gauge case 8 reads the local player's sharpness tier
    (0x61912c, [
        240, 79, 45, 233, 36, 208, 77, 226,
        0, 64, 160, 225, 156, 84, 253, 235,
        16, 3, 159, 229, 76, 128, 132, 226,
        0, 96, 160, 227, 1, 160, 160, 227,
    ]),
    // main+0x276278: sPlayer::getMainPlayer; documented local-player lookup helper
    (0x276278, [
        16, 64, 45, 233, 0, 64, 160, 225,
        20, 0, 159, 229, 0, 0, 159, 231,
        0, 0, 144, 229, 65, 8, 6, 235,
        0, 1, 132, 224, 28, 0, 144, 229,
    ]),
    // main+0x3f8398: local hunter index reader over the session singleton
    (0x3f8398, [
        48, 0, 144, 229, 30, 255, 47, 225,
        48, 16, 128, 229, 30, 255, 47, 225,
        64, 16, 144, 229, 1, 16, 129, 226,
        64, 16, 128, 229, 92, 16, 159, 229,
    ]),
    // main+0x2ad288: uPlayerQuest::adjustSharpness; the only writer of mSharpness
    (0x2ad288, [
        240, 72, 45, 233, 2, 96, 160, 225,
        1, 80, 160, 225, 0, 64, 160, 225,
        140, 58, 255, 235, 1, 0, 80, 227,
        121, 0, 0, 26, 4, 0, 160, 225,
    ]),
    // main+0x26893c: uPlayerBase::getSharpnessType; returns mSharpnessType verbatim
    (0x26893c, [
        240, 19, 1, 227, 1, 0, 144, 231,
        30, 255, 47, 225, 0, 10, 183, 238,
        30, 255, 47, 225, 0, 0, 160, 227,
        30, 255, 47, 225, 0, 0, 160, 227,
    ]),
    // main+0x29cdfc: uPlayerBase::calculateSharpnessType
    (0x29cdfc, [
        240, 72, 45, 233, 1, 64, 160, 225,
        1, 16, 160, 227, 0, 96, 160, 225,
        118, 126, 255, 235, 0, 80, 160, 227,
        0, 0, 80, 227, 38, 0, 0, 26,
    ]),
    // main+0xa4faf0: cGUIObjTexture setTexture
    (0xa4faf0, [
        16, 64, 45, 233, 0, 64, 160, 225,
        120, 0, 148, 229, 0, 0, 80, 227,
        12, 0, 0, 10, 0, 33, 148, 229,
        1, 0, 82, 225, 16, 128, 189, 8,
    ]),
    // main+0xa4fb58: cGUIObjTexture setTextureRect
    (0xa4fb58, [
        16, 64, 45, 233, 0, 64, 160, 225,
        0, 10, 145, 237, 67, 26, 148, 237,
        192, 26, 180, 238, 16, 250, 241, 238,
        14, 0, 0, 26, 1, 10, 145, 237,
    ]),
    // main+0xa4ba20: cGUIObjScissorMask Enable setter
    (0xa4ba20, [
        164, 32, 144, 229, 31, 32, 192, 231,
        17, 32, 192, 231, 164, 32, 128, 229,
        30, 255, 47, 225, 216, 42, 193, 225,
        240, 32, 192, 225, 30, 255, 47, 225,
    ]),
    // main+0xa4ba40: cGUIObjScissorMask Size setter
    (0xa4ba40, [
        48, 72, 45, 233, 216, 74, 192, 225,
        208, 32, 193, 225, 5, 48, 35, 224,
        4, 80, 34, 224, 3, 48, 149, 225,
        48, 136, 189, 8, 168, 32, 128, 229,
    ]),
];
