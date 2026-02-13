use crate::FullscreenShader;
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{binding_types::texture_2d_multisampled, *},
    renderer::RenderDevice,
    RenderApp, RenderStartup,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

/// Adds support for specialized resolve pipelines,
/// which can be used to resolve multisampled texture to another.
pub struct ResolvePlugin;

impl Plugin for ResolvePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "resolve.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .allow_ambiguous_resource::<SpecializedRenderPipelines<ResolvePipeline>>()
            .init_resource::<SpecializedRenderPipelines<ResolvePipeline>>()
            .add_systems(RenderStartup, init_resolve_pipeline);
    }
}

#[derive(Resource)]
pub struct ResolvePipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub fullscreen_shader: FullscreenShader,
    pub fragment_shader: Handle<Shader>,
}

pub fn init_resolve_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "resolve_bind_group_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::FRAGMENT,
            texture_2d_multisampled(TextureSampleType::Float { filterable: false }),
        ),
    );

    commands.insert_resource(ResolvePipeline {
        layout,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "resolve.wgsl"),
    });
}

impl ResolvePipeline {
    pub fn create_bind_group(
        &self,
        render_device: &RenderDevice,
        src_texture: &TextureView,
        pipeline_cache: &PipelineCache,
    ) -> BindGroup {
        render_device.create_bind_group(
            None,
            &pipeline_cache.get_bind_group_layout(&self.layout),
            &BindGroupEntries::single(src_texture),
        )
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct ResolvePipelineKey {
    pub texture_format: TextureFormat,
    pub samples: u32,
}

impl SpecializedRenderPipeline for ResolvePipeline {
    type Key = ResolvePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("resolve pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec![ShaderDefVal::UInt("SAMPLE_COUNT".into(), key.samples)],
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}
