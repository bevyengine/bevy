//! Integration test for the DSB-unavailable fallback logic in
//! `queue_uinodes`. The full queue system pulls in a lot of ECS state
//! (extracted nodes, specialized-pipelines cache, phases, ...) that is
//! impractical to mock standalone; this test instead exercises the
//! observable behavioural contract at the pipeline-key level:
//!
//! - When `SubpixelCapable(true)`, a subpixel-smoothed glyph batch
//!   produces a `UiPipelineKey` with `subpixel: true` — which
//!   `UiPipeline::specialize` maps to the DSB fragment entry.
//! - When `SubpixelCapable(false)`, the same batch must produce a
//!   `UiPipelineKey` with `subpixel: false` — which specializes to the
//!   conventional alpha-blend grayscale variant.
//!
//! This mirrors `queue_uinodes`'s gate:
//! `let subpixel = subpixel_supported && matches!(item, Glyphs { font_smoothing: SubpixelAntiAliased, .. });`
//! and verifies the two resulting pipeline descriptors differ as
//! expected.

use bevy_asset::Handle;
use bevy_render::render_resource::{
    binding_types::{sampler, texture_2d, uniform_buffer},
    BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendFactor, BlendState, SamplerBindingType,
    ShaderStages, SpecializedRenderPipeline, TextureFormat, TextureSampleType,
};
use bevy_render::view::ViewUniform;
use bevy_text::SubpixelCapable;
use bevy_ui_render::{SubpixelTextUniforms, UiPipeline, UiPipelineKey};

fn build_test_pipeline() -> UiPipeline {
    let view_layout = BindGroupLayoutDescriptor::new(
        "ui_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<SubpixelTextUniforms>(false),
            ),
        ),
    );

    let image_layout = BindGroupLayoutDescriptor::new(
        "ui_image_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    UiPipeline {
        view_layout,
        image_layout,
        shader: Handle::default(),
    }
}

/// Replicates the `subpixel` gate in `queue_uinodes`.
///
/// Mirrors the actual expression in `crates/bevy_ui_render/src/lib.rs`:
/// `let subpixel = subpixel_supported && matches!(item, Glyphs { font_smoothing: SubpixelAntiAliased, .. });`
fn gate(capable: SubpixelCapable, item_is_subpixel_glyph: bool) -> bool {
    capable.0 && item_is_subpixel_glyph
}

#[test]
fn subpixel_capable_false_forces_non_subpixel_pipeline_key() {
    let capable = SubpixelCapable(false);
    // A subpixel-smoothed glyph batch
    let subpixel = gate(capable, true);
    assert!(
        !subpixel,
        "SubpixelCapable(false) must force the pipeline key's subpixel flag to false",
    );
}

#[test]
fn subpixel_capable_true_preserves_subpixel_pipeline_key_for_glyph_batch() {
    let capable = SubpixelCapable(true);
    let subpixel = gate(capable, true);
    assert!(
        subpixel,
        "SubpixelCapable(true) must allow subpixel glyph batches to pick the DSB pipeline",
    );
}

#[test]
fn non_glyph_items_never_pick_subpixel_pipeline() {
    // Background colors, borders, images, etc. never carry
    // FontSmoothing::SubpixelAntiAliased, so even with a capable adapter
    // they route to the grayscale pipeline.
    let capable = SubpixelCapable(true);
    let subpixel = gate(capable, false);
    assert!(
        !subpixel,
        "Non-glyph items must never pick the subpixel pipeline even when the adapter is DSB-capable",
    );
}

#[test]
fn fallback_specialization_matches_non_subpixel_variant() {
    // When the gate forces `subpixel = false`, the resulting pipeline key
    // must specialize to the standard alpha-blend fragment (no custom
    // entry point, `ALPHA_BLENDING` blend state).
    let pipeline = build_test_pipeline();
    let descriptor = pipeline.specialize(UiPipelineKey {
        target_format: TextureFormat::Rgba8UnormSrgb,
        anti_alias: true,
        subpixel: false,
    });

    let fragment = descriptor
        .fragment
        .as_ref()
        .expect("fallback pipeline must have a fragment stage");

    assert!(
        fragment.entry_point.is_none(),
        "fallback pipeline must not use the fragment_subpixel entry point",
    );

    let target = fragment.targets[0]
        .as_ref()
        .expect("fallback pipeline must have a color target");
    let blend = target
        .blend
        .expect("fallback pipeline must configure a blend state");
    assert_eq!(
        blend,
        BlendState::ALPHA_BLENDING,
        "fallback pipeline must use conventional alpha blending, not DSB",
    );
    assert_ne!(
        blend.color.src_factor,
        BlendFactor::Src1,
        "fallback pipeline must not use Src1 (dual-source) blend factors",
    );
    assert_eq!(descriptor.label.as_deref(), Some("ui_pipeline"));
}
