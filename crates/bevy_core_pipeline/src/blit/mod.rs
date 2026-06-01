use crate::FullscreenShader;
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::CompositingSpace;
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, texture_2d_array},
        *,
    },
    renderer::RenderDevice,
    GpuResourceAppExt, RenderApp, RenderStartup,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

/// Adds support for specialized "blit pipelines", which can be used to write one texture to another.
pub struct BlitPlugin;

impl Plugin for BlitPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "blit.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .allow_ambiguous_resource::<SpecializedRenderPipelines<BlitPipeline>>()
            .init_gpu_resource::<SpecializedRenderPipelines<BlitPipeline>>()
            .add_systems(RenderStartup, init_blit_pipeline);
    }
}

#[derive(Resource)]
pub struct BlitPipeline {
    /// Bind-group layout for blitting a single-layer source texture.
    pub layout: BindGroupLayoutDescriptor,
    /// Bind-group layout for blitting a multi-layer (multiview) source texture.
    /// The texture binding is a `texture_2d_array` whose layer is selected by
    /// `@builtin(view_index)` in the fragment shader.
    pub layout_multiview: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    pub fullscreen_shader: FullscreenShader,
    pub fragment_shader: Handle<Shader>,
}

pub fn init_blit_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "blit_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: false }),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    );

    let layout_multiview = BindGroupLayoutDescriptor::new(
        "blit_bind_group_layout_multiview",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d_array(TextureSampleType::Float { filterable: false }),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    commands.insert_resource(BlitPipeline {
        layout,
        layout_multiview,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "blit.wgsl"),
    });
}

impl BlitPipeline {
    /// Create a bind group for a blit source texture. `multiview_view_count`
    /// is the source's layer count (`> 1` selects the `texture_2d_array`
    /// layout; `1` selects the single-layer layout).
    pub fn create_bind_group(
        &self,
        render_device: &RenderDevice,
        src_texture: &TextureView,
        pipeline_cache: &PipelineCache,
        multiview_view_count: u32,
    ) -> BindGroup {
        let layout = if multiview_view_count > 1 {
            &self.layout_multiview
        } else {
            &self.layout
        };
        render_device.create_bind_group(
            None,
            &pipeline_cache.get_bind_group_layout(layout),
            &BindGroupEntries::sequential((src_texture, &self.sampler)),
        )
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct BlitPipelineKey {
    pub target_format: TextureFormat,
    pub blend_state: Option<BlendState>,
    pub samples: u32,
    /// Color space of the source texture. When `Some(Srgb)` or `Some(Oklab)`, the blit converts
    /// to linear RGB before writing to the output target.
    pub source_space: Option<CompositingSpace>,
    /// Number of layers in the source texture (1 for single-view; `> 1`
    /// selects the multiview bind-group layout and emits `MULTIVIEW` +
    /// `MAX_VIEW_COUNT` shader-defs into the fragment stage).
    pub multiview_view_count: u32,
}

impl SpecializedRenderPipeline for BlitPipeline {
    type Key = BlitPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        match key.source_space {
            Some(CompositingSpace::Srgb) => shader_defs.push("SRGB_TO_LINEAR".into()),
            Some(CompositingSpace::Oklab) => shader_defs.push("OKLAB_TO_LINEAR".into()),
            Some(CompositingSpace::Linear) | None => {}
        }

        let layout = if key.multiview_view_count > 1 {
            shader_defs.push("MULTIVIEW".into());
            shader_defs.push(ShaderDefVal::UInt(
                "MAX_VIEW_COUNT".into(),
                key.multiview_view_count,
            ));
            self.layout_multiview.clone()
        } else {
            self.layout.clone()
        };

        RenderPipelineDescriptor {
            label: Some("blit pipeline".into()),
            layout: vec![layout],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: key.blend_state,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            multisample: MultisampleState {
                count: key.samples,
                ..default()
            },
            ..default()
        }
    }
}
