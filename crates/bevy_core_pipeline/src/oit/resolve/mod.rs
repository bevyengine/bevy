use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle};
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{storage_buffer_sized, texture_depth_2d, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendComponent,
        BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
        MultisampleState, PipelineCache, PrimitiveState, RenderPipelineDescriptor, Shader,
        ShaderStages, TextureFormat,
    },
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use bevy_utils::HashMap;

use crate::{
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    oit::OrderIndependentTransparencySettings,
};

use super::OitBuffers;

pub const OIT_RESOLVE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(7698420424769536);

pub mod node;

pub struct OitResolvePlugin;
impl Plugin for OitResolvePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            OIT_RESOLVE_SHADER_HANDLE,
            "oit_resolve.wgsl",
            Shader::from_wgsl
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            (
                queue_oit_resolve_pipeline.in_set(RenderSet::Queue),
                prepare_oit_resolve_bind_group.in_set(RenderSet::PrepareBindGroups),
            ),
        );
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<OitResolvePipeline>();
    }
}

#[derive(Resource, Deref)]
pub struct OitResolveBindGroup(pub BindGroup);

#[derive(Resource)]
pub struct OitResolvePipeline {
    pub view_bind_group_layout: BindGroupLayout,
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

/// This key is used to cache the pipeline id and to specialize the render pipeline descriptor
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OitResolvePipelineKey {
    hdr: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_oit_resolve_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    resolve_pipeline: Res<OitResolvePipeline>,
    views: Query<(Entity, &ExtractedView), With<OrderIndependentTransparencySettings>>,
    // Store the key with the id to make the clean up logic easier
    // This also means it will always replace the entry if the key changes so nothing to clean up
    mut cached_pipeline_id: Local<HashMap<Entity, (OitResolvePipelineKey, CachedRenderPipelineId)>>,
) {
    let mut current_view_entities = vec![];
    for (e, view) in &views {
        current_view_entities.push(e);
        let key = OitResolvePipelineKey { hdr: view.hdr };

        if let Some((cached_key, id)) = cached_pipeline_id.get(&e) {
            if *cached_key == key {
                commands.entity(e).insert(OitResolvePipelineId(*id));
                continue;
            }
        }

        let desc = specialize_oit_resolve_pipeline(key, &resolve_pipeline);

        let pipeline_id = pipeline_cache.queue_render_pipeline(desc);
        commands.entity(e).insert(OitResolvePipelineId(pipeline_id));
        cached_pipeline_id.insert(e, (key, pipeline_id));
    }

    // Clear cache for views that don't exist anymore
    // We can't rely on removal detection here because components in the render world aren't persisted
    for e in cached_pipeline_id.keys().copied().collect::<Vec<_>>() {
        if !current_view_entities.contains(&e) {
            cached_pipeline_id.remove(&e);
        }
    }
}

fn specialize_oit_resolve_pipeline(
    key: OitResolvePipelineKey,
    resolve_pipeline: &OitResolvePipeline,
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
            entry_point: "fragment".into(),
            shader: OIT_RESOLVE_SHADER_HANDLE,
            shader_defs: vec![],
            targets: vec![Some(ColorTargetState {
                format,
                blend: Some(BlendState {
                    color: BlendComponent::OVER,
                    alpha: BlendComponent::OVER,
                }),
                write_mask: ColorWrites::ALL,
            })],
        }),
        vertex: fullscreen_shader_vertex_state(),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
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
