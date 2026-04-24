//! Integration tests for the UI subpixel-text DSB pipeline variant.
//!
//! Verifies that `UiPipeline::specialize` branches to the dual-source-blend
//! fragment entry point + blend state when `UiPipelineKey.subpixel == true`,
//! and preserves the existing alpha-blend path otherwise.

use bevy_asset::Handle;
use bevy_render::render_resource::{
    binding_types::{sampler, texture_2d, uniform_buffer},
    BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendFactor, BlendOperation, BlendState,
    SamplerBindingType, ShaderStages, SpecializedRenderPipeline, TextureFormat, TextureSampleType,
};
use bevy_render::view::ViewUniform;
use bevy_shader::ShaderDefVal;
use bevy_ui_render::{SubpixelTextUniforms, UiPipeline, UiPipelineKey};

fn has_subpixel_def(def: &ShaderDefVal) -> bool {
    matches!(def, ShaderDefVal::Bool(name, true) if name == "SUBPIXEL")
}

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

#[test]
fn subpixel_variant_selects_dsb_fragment_entry() {
    let pipeline = build_test_pipeline();

    let descriptor = pipeline.specialize(UiPipelineKey {
        target_format: TextureFormat::Rgba8UnormSrgb,
        anti_alias: true,
        subpixel: true,
    });

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
        Some("ui_pipeline_subpixel"),
        "subpixel variant should use the ui_pipeline_subpixel label",
    );
}

#[test]
fn non_subpixel_variant_keeps_alpha_blend_fragment() {
    let pipeline = build_test_pipeline();

    let descriptor = pipeline.specialize(UiPipelineKey {
        target_format: TextureFormat::Rgba8UnormSrgb,
        anti_alias: true,
        subpixel: false,
    });

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
        Some("ui_pipeline"),
        "non-subpixel variant should use the ui_pipeline label",
    );
}

#[test]
fn subpixel_key_differs_from_non_subpixel_key() {
    // Cache keys must differ between variants so `SpecializedRenderPipelines`
    // keeps both descriptors alive in the same frame.
    let a = UiPipelineKey {
        target_format: TextureFormat::Rgba8UnormSrgb,
        anti_alias: true,
        subpixel: false,
    };
    let b = UiPipelineKey {
        target_format: TextureFormat::Rgba8UnormSrgb,
        anti_alias: true,
        subpixel: true,
    };
    assert_ne!(a, b);
}
