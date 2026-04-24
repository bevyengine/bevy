//! Unit tests for the shared subpixel tuning resources defined in
//! `crates/bevy_text/src/subpixel.rs`.
//!
//! Asserts:
//! - [`SubpixelTextSettings::default`] matches the GPUI-derived gamma=1.8
//!   preset the render crates expect.
//! - [`SubpixelLcdLayout::default`] is `HorizontalRgb` and `pack_u32`
//!   discriminants match the `SUBPIXEL_LAYOUT_*` constants in
//!   `bevy_ui_render`'s `ui.wgsl`.
//! - [`SubpixelCapable`] wraps a `bool` and is constructible.

use bevy_math::Vec4;
use bevy_text::{SubpixelCapable, SubpixelLcdLayout, SubpixelTextSettings};

#[test]
fn subpixel_text_settings_default_matches_gpui_gamma_1_8() {
    let settings = SubpixelTextSettings::default();
    assert!(
        (settings.enhanced_contrast - 0.5).abs() < f32::EPSILON,
        "enhanced_contrast default should be GPUI's 0.5, got {}",
        settings.enhanced_contrast,
    );
    assert_eq!(
        settings.gamma_ratios,
        Vec4::new(0.14746, -0.89481, 1.47021, -0.32474),
        "gamma_ratios default should match the gamma=1.8 row of GPUI's GAMMA_INCORRECT_TARGET_RATIOS",
    );
}

#[test]
fn subpixel_lcd_layout_default_is_horizontal_rgb() {
    assert_eq!(
        SubpixelLcdLayout::default(),
        SubpixelLcdLayout::HorizontalRgb
    );
}

#[test]
fn subpixel_lcd_layout_pack_u32_matches_shader_constants() {
    // These numeric values must stay in sync with `SUBPIXEL_LAYOUT_*`
    // constants in `bevy_ui_render/src/ui.wgsl` (and, post-phase-06,
    // `bevy_sprite_render/src/sprite.wgsl`).
    assert_eq!(SubpixelLcdLayout::HorizontalRgb.pack_u32(), 0);
    assert_eq!(SubpixelLcdLayout::HorizontalBgr.pack_u32(), 1);
}

#[test]
fn subpixel_capable_is_copy_bool_wrapper() {
    let yes = SubpixelCapable(true);
    let no = SubpixelCapable(false);
    assert!(yes.0);
    assert!(!no.0);
    // Copy, not Clone — the Copy derive means rebinding after move is ok.
    let also_yes = yes;
    assert!(yes.0);
    assert!(also_yes.0);
}

#[test]
fn subpixel_text_settings_can_be_overridden() {
    let custom = SubpixelTextSettings {
        enhanced_contrast: 0.35,
        gamma_ratios: Vec4::new(0.1, 0.2, 0.3, 0.4),
    };
    assert!((custom.enhanced_contrast - 0.35).abs() < f32::EPSILON);
    assert_eq!(custom.gamma_ratios, Vec4::new(0.1, 0.2, 0.3, 0.4));
}
