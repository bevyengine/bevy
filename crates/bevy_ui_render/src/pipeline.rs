use crate::SubpixelTextUniforms;
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_ecs::prelude::*;
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    view::ViewUniform,
};
use bevy_shader::Shader;
use bevy_text::SubpixelCapable;
use bevy_utils::default;

#[derive(Resource)]
pub struct UiPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub image_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
}

pub fn init_ui_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Binding 1 is the `SubpixelTextUniforms` uniform (see
    // [`crate::SubpixelTextUniforms`]). It is declared on every UI pipeline
    // variant — including the non-subpixel path — so the view bind group
    // layout is a single shared definition. The non-subpixel WGSL entry
    // points simply don't reference the binding; naga/wgpu are fine with
    // unused bindings as long as the layout matches. The uniform buffer is
    // populated by [`crate::prepare_uinodes`] from the main-world
    // `SubpixelTextSettings` / `SubpixelLcdLayout` resources.
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

    commands.insert_resource(UiPipeline {
        view_layout,
        image_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "ui.wgsl"),
    });
}

/// Populate the [`SubpixelCapable`] resource from the active render adapter's
/// wgpu feature set at `RenderStartup`. Reads
/// [`WgpuFeatures::DUAL_SOURCE_BLENDING`] — the feature the subpixel UI
/// fragment entry (`fragment_subpixel` in `ui.wgsl`) requires for its
/// `@blend_src(1)` output.
///
/// On adapters without DSB support, [`crate::queue_uinodes`] downgrades
/// glyph batches tagged [`bevy_text::FontSmoothing::SubpixelAntiAliased`] to
/// the grayscale pipeline variant — no panic, no shader error, just a
/// silent visual degradation to approximate grayscale AA.
///
/// `bevy_sprite_render` (phase-06) will add its own
/// `init_sprite_subpixel_capability` system that writes the same resource
/// from the same underlying feature set; the two systems are idempotent.
pub fn init_ui_subpixel_capability(mut commands: Commands, render_device: Res<RenderDevice>) {
    let supported = render_device
        .features()
        .contains(WgpuFeatures::DUAL_SOURCE_BLENDING);
    commands.insert_resource(SubpixelCapable(supported));
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct UiPipelineKey {
    pub target_format: TextureFormat,
    pub anti_alias: bool,
    /// Whether this pipeline variant targets RGB subpixel text glyphs.
    ///
    /// When `true`, [`UiPipeline::specialize`] emits the dual-source-blend
    /// variant (`fragment_subpixel` entry point, `SUBPIXEL` shader def,
    /// `Src1` / `OneMinusSrc1` blend factors). Queue-side selection in
    /// [`crate::queue_uinodes`] sets this for glyph batches whose source
    /// section used `FontSmoothing::SubpixelAntiAliased`, but only when
    /// the [`bevy_text::SubpixelCapable`] resource reports `true` (the
    /// active adapter advertises `wgpu::Features::DUAL_SOURCE_BLENDING`).
    /// On non-DSB adapters the flag is always forced to `false` so glyphs
    /// silently fall back to the grayscale pipeline.
    pub subpixel: bool,
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
                // mode
                VertexFormat::Uint32,
                // border radius
                VertexFormat::Float32x4,
                // border thickness
                VertexFormat::Float32x4,
                // border size
                VertexFormat::Float32x2,
                // position relative to the center
                VertexFormat::Float32x2,
            ],
        );
        let mut shader_defs = Vec::new();
        if key.anti_alias {
            shader_defs.push("ANTI_ALIAS".into());
        }
        if key.subpixel {
            shader_defs.push("SUBPIXEL".into());
        }

        // Subpixel text renders via a dual-source-blend fragment shader so the
        // framebuffer can consume a per-channel alpha from `@blend_src(1)`.
        // The color blend is
        //   result.rgb = fg.rgb * alpha_per_channel
        //              + dst.rgb * (1 - alpha_per_channel)
        // which requires `Src1` / `OneMinusSrc1` (the per-channel alpha is the
        // dual-source output). The alpha component keeps conventional
        // premultiplied behavior so the UI framebuffer's own alpha stays
        // sensible. Mirrors GPUI's subpixel pipeline
        // (`zed/crates/gpui/src/platform/blade/blade_renderer.rs`) and the
        // cosmic-era fork precedent (`subpixel-text-followups` commit
        // `aeb7aadfb`).
        let (fragment_entry_point, blend) = if key.subpixel {
            (
                Some("fragment_subpixel".into()),
                BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Src1,
                        dst_factor: BlendFactor::OneMinusSrc1,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                },
            )
        } else {
            (None, BlendState::ALPHA_BLENDING)
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: fragment_entry_point,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(blend),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.image_layout.clone()],
            label: Some(if key.subpixel {
                "ui_pipeline_subpixel".into()
            } else {
                "ui_pipeline".into()
            }),
            ..default()
        }
    }
}
