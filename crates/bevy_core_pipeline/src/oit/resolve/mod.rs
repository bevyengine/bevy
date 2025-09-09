use super::OitBuffers;
use crate::{oit::OrderIndependentTransparencySettings, FullscreenShader};
use bevy_app::Plugin;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer};
use bevy_derive::Deref;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
};
use bevy_image::BevyDefault as _;
use bevy_render::{
    render_resource::{
        binding_types::{storage_buffer_sized, texture_depth_2d, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendComponent,
        BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, DownlevelFlags,
        FragmentState, PipelineCache, RenderPipelineDescriptor, ShaderStages, TextureFormat,
    },
    renderer::{RenderAdapter, RenderDevice},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::ShaderDefVal;
use bevy_utils::default;
use tracing::warn;

/// Contains the render node used to run the resolve pass.
pub mod node;

/// Minimum required value of `wgpu::Limits::max_storage_buffers_per_shader_stage`.
pub const OIT_REQUIRED_STORAGE_BUFFERS: u32 = 2;

/// Plugin needed to resolve the Order Independent Transparency (OIT) buffer to the screen.
pub struct OitResolvePlugin;
impl Plugin for OitResolvePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        embedded_asset!(app, "oit_resolve.wgsl");
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if !is_oit_supported(
            render_app.world().resource::<RenderAdapter>(),
            render_app.world().resource::<RenderDevice>(),
            true,
        ) {
            return;
        }

        render_app
            .add_systems(
                Render,
                (
                    queue_oit_resolve_pipeline.in_set(RenderSystems::Queue),
                    prepare_oit_resolve_bind_group.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .init_resource::<OitResolvePipeline>();
    }
}

pub fn is_oit_supported(adapter: &RenderAdapter, device: &RenderDevice, warn: bool) -> bool {
    if !adapter
        .get_downlevel_capabilities()
        .flags
        .contains(DownlevelFlags::FRAGMENT_WRITABLE_STORAGE)
    {
        if warn {
            warn!("OrderIndependentTransparencyPlugin not loaded. GPU lacks support: DownlevelFlags::FRAGMENT_WRITABLE_STORAGE.");
        }
        return false;
    }

    let max_storage_buffers_per_shader_stage = device.limits().max_storage_buffers_per_shader_stage;

    if max_storage_buffers_per_shader_stage < OIT_REQUIRED_STORAGE_BUFFERS {
        if warn {
            warn!(
                max_storage_buffers_per_shader_stage,
                OIT_REQUIRED_STORAGE_BUFFERS,
                "OrderIndependentTransparencyPlugin not loaded. RenderDevice lacks support: max_storage_buffers_per_shader_stage < OIT_REQUIRED_STORAGE_BUFFERS."
            );
        }
        return false;
    }

    true
}

/// Bind group for the OIT resolve pass.
#[derive(Resource, Deref)]
pub struct OitResolveBindGroup(pub BindGroup);

/// Bind group layouts used for the OIT resolve pass.
#[derive(Resource)]
pub struct OitResolvePipeline {
    /// View bind group layout.
    pub view_bind_group_layout: BindGroupLayout,
    /// Depth bind group layout.
    pub oit_depth_bind_group_layout: BindGroupLayout,
}

impl FromWorld for OitResolvePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_bind_group_layout = render_device.create_bind_group_layout(
            "oit_resolve_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    // layers
                    storage_buffer_sized(false, None),
                    // layer ids
                    storage_buffer_sized(false, None),
                ),
            ),
        );

        let oit_depth_bind_group_layout = render_device.create_bind_group_layout(
            "oit_depth_bind_group_layout",
            &BindGroupLayoutEntries::single(ShaderStages::FRAGMENT, texture_depth_2d()),
        );
        OitResolvePipeline {
            view_bind_group_layout,
            oit_depth_bind_group_layout,
        }
    }
}

#[derive(Component, Deref, Clone, Copy)]
pub struct OitResolvePipelineId(pub CachedRenderPipelineId);

/// This key is used to cache the pipeline id and to specialize the render pipeline descriptor.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OitResolvePipelineKey {
    hdr: bool,
    layer_count: i32,
}

pub fn queue_oit_resolve_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    resolve_pipeline: Res<OitResolvePipeline>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            &OrderIndependentTransparencySettings,
        ),
        With<OrderIndependentTransparencySettings>,
    >,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
    // Store the key with the id to make the clean up logic easier.
    // This also means it will always replace the entry if the key changes so nothing to clean up.
    mut cached_pipeline_id: Local<EntityHashMap<(OitResolvePipelineKey, CachedRenderPipelineId)>>,
) {
    let mut current_view_entities = EntityHashSet::default();
    for (e, view, oit_settings) in &views {
        current_view_entities.insert(e);
        let key = OitResolvePipelineKey {
            hdr: view.hdr,
            layer_count: oit_settings.layer_count,
        };

        if let Some((cached_key, id)) = cached_pipeline_id.get(&e)
            && *cached_key == key
        {
            commands.entity(e).insert(OitResolvePipelineId(*id));
            continue;
        }

        let desc = specialize_oit_resolve_pipeline(
            key,
            &resolve_pipeline,
            &fullscreen_shader,
            &asset_server,
        );

        let pipeline_id = pipeline_cache.queue_render_pipeline(desc);
        commands.entity(e).insert(OitResolvePipelineId(pipeline_id));
        cached_pipeline_id.insert(e, (key, pipeline_id));
    }

    // Clear cache for views that don't exist anymore.
    for e in cached_pipeline_id.keys().copied().collect::<Vec<_>>() {
        if !current_view_entities.contains(&e) {
            cached_pipeline_id.remove(&e);
        }
    }
}

fn specialize_oit_resolve_pipeline(
    key: OitResolvePipelineKey,
    resolve_pipeline: &OitResolvePipeline,
    fullscreen_shader: &FullscreenShader,
    asset_server: &AssetServer,
) -> RenderPipelineDescriptor {
    let format = if key.hdr {
        ViewTarget::TEXTURE_FORMAT_HDR
    } else {
        TextureFormat::bevy_default()
    };

    RenderPipelineDescriptor {
        label: Some("oit_resolve_pipeline".into()),
        layout: vec![
            resolve_pipeline.view_bind_group_layout.clone(),
            resolve_pipeline.oit_depth_bind_group_layout.clone(),
        ],
        fragment: Some(FragmentState {
            shader: load_embedded_asset!(asset_server, "oit_resolve.wgsl"),
            shader_defs: vec![ShaderDefVal::UInt(
                "LAYER_COUNT".into(),
                key.layer_count as u32,
            )],
            targets: vec![Some(ColorTargetState {
                format,
                blend: Some(BlendState {
                    color: BlendComponent::OVER,
                    alpha: BlendComponent::OVER,
                }),
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        vertex: fullscreen_shader.to_vertex_state(),
        ..default()
    }
}

pub fn prepare_oit_resolve_bind_group(
    mut commands: Commands,
    resolve_pipeline: Res<OitResolvePipeline>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    buffers: Res<OitBuffers>,
) {
    if let (Some(binding), Some(layers_binding), Some(layer_ids_binding)) = (
        view_uniforms.uniforms.binding(),
        buffers.layers.binding(),
        buffers.layer_ids.binding(),
    ) {
        let bind_group = render_device.create_bind_group(
            "oit_resolve_bind_group",
            &resolve_pipeline.view_bind_group_layout,
            &BindGroupEntries::sequential((binding.clone(), layers_binding, layer_ids_binding)),
        );
        commands.insert_resource(OitResolveBindGroup(bind_group));
    }
}
