//! Shared resources for RGB subpixel antialiased text rendering.
//!
//! These types live in `bevy_text` so both `bevy_ui_render` (for UI overlay
//! text) and `bevy_sprite_render` (for world-space `Text2d`) can share a
//! single set of tuning knobs — app authors configure subpixel rendering
//! once for both pipelines.
//!
//! `bevy_text` intentionally does not depend on `bevy_render`, so the GPU-
//! facing wiring (uniforms, bind groups, shader code) stays in the render
//! crates. Each render crate maps these plain-data resources onto its own
//! `ShaderType` uniform (`SubpixelTextUniforms` in `bevy_ui_render`) at
//! render prep time.
//!
//! Cosmic-era predecessor: `7d7773720:crates/bevy_text/src/subpixel.rs` on
//! the fork branch `subpixel-text-followups`. The shape carries forward; the
//! `SubpixelLcdLayout` enum has been pared back to the two genuinely
//! functional variants (see the type's docs).

use bevy_ecs::prelude::ReflectResource;
use bevy_ecs::resource::Resource;
use bevy_math::Vec4;
use bevy_reflect::{prelude::ReflectDefault, Reflect};

/// Tracks whether the active wgpu adapter exposes
/// [`wgpu::Features::DUAL_SOURCE_BLENDING`](https://docs.rs/wgpu/latest/wgpu/struct.Features.html#associatedconstant.DUAL_SOURCE_BLENDING),
/// which [`FontSmoothing::SubpixelAntiAliased`](crate::FontSmoothing::SubpixelAntiAliased)
/// requires for its dual-source-blend shader in both `bevy_ui_render` and
/// `bevy_sprite_render`.
///
/// Inserted once at render startup by `bevy_ui_render::init_ui_subpixel_capability`
/// (and, once phase-06 lands, `bevy_sprite_render::init_sprite_subpixel_capability`);
/// either system produces the same value (both read the same adapter feature
/// set), so the resource is consistent regardless of which render sub-app runs
/// first.
///
/// When `false`, the subpixel queue paths in both renderers transparently
/// fall back to the grayscale pipeline variant — the RGBA coverage atlas is
/// still sampled, but the non-subpixel fragment entry only uses one channel
/// as alpha, so glyphs render as approximate grayscale AA without panicking.
///
/// Consumers should read via `Option<Res<SubpixelCapable>>` and treat `None`
/// as `SubpixelCapable(false)`, because `TextPlugin` does not insert this
/// resource (only render plugins do). That keeps `bevy_text` usable in
/// headless contexts without a render app.
#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct SubpixelCapable(pub bool);

/// Tuning parameters for RGB subpixel antialiased text rendering.
///
/// Only consulted when [`FontSmoothing::SubpixelAntiAliased`](crate::FontSmoothing::SubpixelAntiAliased)
/// is active and [`SubpixelCapable`] is `true`. Defaults match GPUI's
/// gamma=1.8 preset, which works well across dark and light UI backgrounds.
///
/// Shared between `bevy_ui_render` (UI overlay text) and `bevy_sprite_render`
/// (world-space `Text2d`). Both pipelines read the same resource so app
/// authors only tune subpixel rendering once.
///
/// App authors tuning for a specific display or background can override:
/// - `enhanced_contrast`: higher values yield more aggressive per-channel
///   gamma; lower values are more muted (useful on very low-contrast
///   backgrounds).
/// - `gamma_ratios`: cubic-polynomial coefficients matching GPUI's
///   `GAMMA_INCORRECT_TARGET_RATIOS` table. Alternate rows of that table
///   correspond to different target gammas (1.0, 1.2, ... 2.2).
///
/// ```
/// use bevy_math::Vec4;
/// use bevy_ecs::prelude::*;
/// use bevy_text::SubpixelTextSettings;
///
/// # let mut world = World::new();
/// world.insert_resource(SubpixelTextSettings {
///     enhanced_contrast: 0.35,
///     gamma_ratios: Vec4::new(0.14746, -0.89481, 1.47021, -0.32474),
/// });
/// ```
#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
pub struct SubpixelTextSettings {
    /// Strength of the per-channel contrast boost applied before gamma
    /// correction. GPUI's default is `0.5`.
    pub enhanced_contrast: f32,
    /// Cubic-polynomial coefficients used by the subpixel gamma correction.
    /// Defaults match GPUI's gamma=1.8 row of `GAMMA_INCORRECT_TARGET_RATIOS`
    /// scaled by `NORM13`/`NORM24`. See
    /// `references/zed/crates/gpui/src/platform.rs::get_gamma_correction_ratios`
    /// for the source table and the derivation.
    pub gamma_ratios: Vec4,
}

impl Default for SubpixelTextSettings {
    fn default() -> Self {
        Self {
            enhanced_contrast: 0.5,
            gamma_ratios: Vec4::new(0.14746, -0.89481, 1.47021, -0.32474),
        }
    }
}

/// Subpixel arrangement of the target LCD panel.
///
/// Defaults to [`SubpixelLcdLayout::HorizontalRgb`] — the arrangement of
/// ~99% of desktop LCDs and nearly all laptop panels. Override for BGR
/// panels (some older displays). Automatic detection of the host panel's
/// layout is deliberately out of scope — each platform's API is fiddly
/// enough to be its own future spec.
///
/// Only consulted when [`FontSmoothing::SubpixelAntiAliased`](crate::FontSmoothing::SubpixelAntiAliased)
/// is active and [`SubpixelCapable`] is `true`.
///
/// Shared between `bevy_ui_render` and `bevy_sprite_render`.
///
/// # Why only the horizontal variants?
///
/// The glyph atlas is produced by swash with
/// [`Format::Subpixel`](https://docs.rs/swash/latest/swash/zeno/enum.Format.html),
/// which emits three coverage values *per logical pixel*, pre-offset along
/// the horizontal subpixel stripe. The atlas therefore already encodes the
/// R-at-left / G-at-center / B-at-right geometry.
///
/// For [`SubpixelLcdLayout::HorizontalRgb`] the shader samples and emits the
/// atlas RGB as-is. For [`SubpixelLcdLayout::HorizontalBgr`] the shader
/// swizzles to `.bgr`, which inverts the color-fringe direction — on a
/// physically BGR panel this yields correct subpixel antialiasing.
///
/// Vertical variants (red-at-top / blue-at-bottom, for rotated portrait
/// panels or vertical-stripe OLEDs) are deliberately deferred: correct
/// vertical-subpixel antialiasing would require either rasterising the
/// glyph with a rotated subpixel direction or maintaining a second
/// vertical-subpixel atlas. A follow-up spec can revisit this.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Resource, Default, Debug, Clone, PartialEq, Hash)]
pub enum SubpixelLcdLayout {
    /// Red at left, green centered, blue at right. Default and most common.
    #[default]
    HorizontalRgb,
    /// Blue at left, green centered, red at right. Some older displays.
    HorizontalBgr,
}

impl SubpixelLcdLayout {
    /// Packed discriminant matching the `SUBPIXEL_LAYOUT_*` constants in
    /// `bevy_ui_render`'s `ui.wgsl` (and, post-phase-06, `bevy_sprite_render`'s
    /// `sprite.wgsl`). Keep the numeric values in sync with those shader
    /// constants.
    pub fn pack_u32(self) -> u32 {
        match self {
            Self::HorizontalRgb => 0,
            Self::HorizontalBgr => 1,
        }
    }
}
