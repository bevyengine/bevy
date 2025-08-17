use crate::FullscreenShader;
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d},
        *,
    },
    renderer::RenderDevice,
    RenderApp, RenderStartup,
};
use bevy_shader::Shader;
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
            .init_resource::<SpecializedRenderPipelines<BlitPipeline>>()
            .add_systems(RenderStartup, init_blit_pipeline);
    }
}

#[derive(Resource)]
pub struct BlitPipeline {
    pub layout: BindGroupLayout,
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
    let layout = render_device.create_bind_group_layout(
        "blit_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: false }),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    commands.insert_resource(BlitPipeline {
        layout,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "blit.wgsl"),
    });
}

impl BlitPipeline {
    pub fn create_bind_group(
        &self,
        render_device: &RenderDevice,
        src_texture: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(
            None,
            &self.layout,
            &BindGroupEntries::sequential((src_texture, &self.sampler)),
        )
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct BlitPipelineKey {
    pub texture_format: TextureFormat,
    pub blend_state: Option<BlendState>,
    pub samples: u32,
}

impl SpecializedRenderPipeline for BlitPipeline {
    type Key = BlitPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("blit pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
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
