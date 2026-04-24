//! Integration tests for the sprite subpixel-text DSB pipeline variant.
//!
//! Verifies that [`SpritePipeline::specialize`] branches to the dual-source-
//! blend fragment entry + blend state when
//! [`SpritePipelineKey::SUBPIXEL`] is set, and preserves the existing
//! alpha-blend path otherwise. Also checks the bit allocation for
//! `SUBPIXEL` does not collide with the `COLOR_TARGET_FORMAT`,
//! `TONEMAP_METHOD`, or `MSAA` reserved regions.
//!
//! Mirrors `bevy_ui_render/tests/subpixel_pipeline.rs` (phase-04 of
//! spec/0013) for the sprite pipeline (phase-06).

use bevy_asset::Handle;
use bevy_render::render_resource::{
    binding_types::{sampler, texture_2d, uniform_buffer},
    BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendFactor, BlendOperation, BlendState,
    SamplerBindingType, ShaderStages, SpecializedRenderPipeline, TextureFormat, TextureSampleType,
};
use bevy_render::view::ViewUniform;
use bevy_shader::ShaderDefVal;
use bevy_sprite_render::{SpritePipeline, SpritePipelineKey, SubpixelTextUniforms};

fn has_subpixel_def(def: &ShaderDefVal) -> bool {
    matches!(def, ShaderDefVal::Bool(name, true) if name == "SUBPIXEL")
}

/// Builds a [`SpritePipeline`] with a layout-only view/material binding so
/// `specialize` can be called without a live `RenderDevice`.
///
/// The layout's tonemapping-LUT bindings use generic `texture_2d` /
/// `sampler` entries rather than the real
/// `get_lut_bind_group_layout_entries` shape; `specialize` does not
/// inspect layout contents, only clones them into the descriptor.
fn build_test_pipeline() -> SpritePipeline {
    let view_layout = BindGroupLayoutDescriptor::new(
        "sprite_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<SubpixelTextUniforms>(false),
            ),
        ),
    );

    let material_layout = BindGroupLayoutDescriptor::new(
        "sprite_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    SpritePipeline {
        view_layout,
        material_layout,
        shader: Handle::default(),
    }
}

#[test]
fn subpixel_variant_selects_dsb_fragment_entry() {
    let pipeline = build_test_pipeline();

    let key = SpritePipelineKey::from_target_format(TextureFormat::Rgba8UnormSrgb)
        | SpritePipelineKey::from_msaa_samples(1)
        | SpritePipelineKey::SUBPIXEL;
    let descriptor = pipeline.specialize(key);

    let fragment = descriptor
        .fragment
        .as_ref()
        .expect("subpixel pipeline must have a fragment stage");

    let entry_point = fragment
        .entry_point
        .as_ref()
        .expect("subpixel pipeline must set a custom fragment entry point");
    assert_eq!(entry_point.as_ref(), "fragment_subpixel");

    // `SUBPIXEL` shader_def should be present on both stages so the WGSL
    // subpixel block compiles.
    assert!(
        fragment.shader_defs.iter().any(has_subpixel_def),
        "SUBPIXEL shader_def missing from fragment: {:?}",
        fragment.shader_defs,
    );
    assert!(
        descriptor.vertex.shader_defs.iter().any(has_subpixel_def),
        "SUBPIXEL shader_def missing from vertex: {:?}",
        descriptor.vertex.shader_defs,
    );

    let target = fragment.targets[0]
        .as_ref()
        .expect("subpixel pipeline must have a color target");
    let blend = target
        .blend
        .expect("subpixel pipeline must configure a blend state");

    assert_eq!(blend.color.src_factor, BlendFactor::Src1);
    assert_eq!(blend.color.dst_factor, BlendFactor::OneMinusSrc1);
    assert_eq!(blend.color.operation, BlendOperation::Add);

    assert_eq!(blend.alpha.src_factor, BlendFactor::One);
    assert_eq!(blend.alpha.dst_factor, BlendFactor::OneMinusSrcAlpha);
    assert_eq!(blend.alpha.operation, BlendOperation::Add);

    assert_eq!(
        descriptor.label.as_deref(),
        Some("sprite_pipeline_subpixel"),
        "subpixel variant should use the sprite_pipeline_subpixel label",
    );
}

#[test]
fn non_subpixel_variant_keeps_alpha_blend_fragment() {
    let pipeline = build_test_pipeline();

    let key = SpritePipelineKey::from_target_format(TextureFormat::Rgba8UnormSrgb)
        | SpritePipelineKey::from_msaa_samples(1);
    let descriptor = pipeline.specialize(key);

    let fragment = descriptor
        .fragment
        .as_ref()
        .expect("non-subpixel pipeline must have a fragment stage");

    assert!(
        fragment.entry_point.is_none(),
        "non-subpixel variant must not override the default fragment entry",
    );
    assert!(
        !fragment.shader_defs.iter().any(has_subpixel_def),
        "SUBPIXEL shader_def leaked into non-subpixel fragment: {:?}",
        fragment.shader_defs,
    );

    let target = fragment.targets[0]
        .as_ref()
        .expect("non-subpixel pipeline must have a color target");
    let blend = target
        .blend
        .expect("non-subpixel pipeline must configure a blend state");

    assert_eq!(blend, BlendState::ALPHA_BLENDING);

    assert_eq!(
        descriptor.label.as_deref(),
        Some("sprite_pipeline"),
        "non-subpixel variant should use the sprite_pipeline label",
    );
}

#[test]
fn subpixel_key_differs_from_non_subpixel_key() {
    // Cache keys must differ between variants so
    // `SpecializedRenderPipelines` keeps both descriptors alive in the
    // same frame.
    let base = SpritePipelineKey::from_target_format(TextureFormat::Rgba8UnormSrgb)
        | SpritePipelineKey::from_msaa_samples(1);
    let subpixel = base | SpritePipelineKey::SUBPIXEL;
    assert_ne!(base, subpixel);
}

#[test]
fn subpixel_bit_does_not_collide_with_reserved_regions() {
    // Ensure the SUBPIXEL bit is outside every pre-existing reserved
    // region of the pipeline-key. A collision would silently corrupt
    // either our subpixel gating or the reserved field.
    let subpixel = SpritePipelineKey::SUBPIXEL;
    assert!(
        (subpixel & SpritePipelineKey::COLOR_TARGET_FORMAT_RESERVED_BITS).is_empty(),
        "SUBPIXEL overlaps the COLOR_TARGET_FORMAT reserved region",
    );
    assert!(
        (subpixel & SpritePipelineKey::MSAA_RESERVED_BITS).is_empty(),
        "SUBPIXEL overlaps the MSAA reserved region",
    );
    assert!(
        (subpixel & SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS).is_empty(),
        "SUBPIXEL overlaps the TONEMAP_METHOD reserved region",
    );

    // It also must not collide with any of the low-bit flags.
    for other in [
        SpritePipelineKey::TONEMAP_IN_SHADER,
        SpritePipelineKey::DEBAND_DITHER,
        SpritePipelineKey::SRGB_COMPOSITING,
        SpritePipelineKey::OKLAB_COMPOSITING,
    ] {
        assert!(
            (subpixel & other).is_empty(),
            "SUBPIXEL collides with {:?}",
            other,
        );
    }
}
