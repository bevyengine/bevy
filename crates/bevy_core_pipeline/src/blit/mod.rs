use core::{ops::Deref, result::Result};

use crate::FullscreenShader;
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d},
        *,
    },
    renderer::RenderDevice,
    RenderApp, RenderStartup,
};
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
            .allow_ambiguous_resource::<BlitPipeline>()
            .add_systems(RenderStartup, init_blit_pipeline);
    }
}

#[derive(Resource)]
pub struct BlitPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub specialized_cache: SpecializedCache<RenderPipeline, BlitSpecializer>,
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

    let base_descriptor = RenderPipelineDescriptor {
        label: Some("blit pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: load_embedded_asset!(asset_server.deref(), "blit.wgsl"),
            ..default()
        }),
        ..default()
    };

    let specialized_cache = SpecializedCache::new(BlitSpecializer, base_descriptor);

    commands.insert_resource(BlitPipeline {
        layout,
        sampler,
        specialized_cache,
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

pub struct BlitSpecializer;

impl Specializer<RenderPipeline> for BlitSpecializer {
    type Key = BlitKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut <RenderPipeline as Specializable>::Descriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.samples;

        descriptor.fragment_mut()?.set_target(
            0,
            ColorTargetState {
                format: key.texture_format,
                blend: key.blend_state,
                write_mask: ColorWrites::ALL,
            },
        );

        Ok(key)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, SpecializerKey)]
pub struct BlitKey {
    pub texture_format: TextureFormat,
    pub blend_state: Option<BlendState>,
    pub samples: u32,
}
