use crate::{
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DeferredPrepass, ViewPrepassTextures},
};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::prelude::*;
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{binding_types::texture_2d, *},
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    Render, RenderApp, RenderSet,
};

use bevy_ecs::query::QueryItem;
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
};

use super::DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT;

pub const COPY_DEFERRED_LIGHTING_ID_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(5230948520734987);
pub struct CopyDeferredLightingIdPlugin;

impl Plugin for CopyDeferredLightingIdPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            COPY_DEFERRED_LIGHTING_ID_SHADER_HANDLE,
            "copy_deferred_lighting_id.wgsl",
            Shader::from_wgsl
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            (prepare_deferred_lighting_id_textures.in_set(RenderSet::PrepareResources),),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<CopyDeferredLightingIdPipeline>();
    }
}

#[derive(Default)]
pub struct CopyDeferredLightingIdNode;
impl CopyDeferredLightingIdNode {
    pub const NAME: &'static str = "copy_deferred_lighting_id";
}

impl ViewNode for CopyDeferredLightingIdNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static DeferredLightingIdDepthTexture,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (_view_target, view_prepass_textures, deferred_lighting_id_depth_texture): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let copy_deferred_lighting_id_pipeline = world.resource::<CopyDeferredLightingIdPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) =
            pipeline_cache.get_render_pipeline(copy_deferred_lighting_id_pipeline.pipeline_id)
        else {
            return Ok(());
        };
        let Some(deferred_lighting_pass_id_texture) =
            &view_prepass_textures.deferred_lighting_pass_id
        else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "copy_deferred_lighting_id_bind_group",
            &copy_deferred_lighting_id_pipeline.layout,
            &BindGroupEntries::single(&deferred_lighting_pass_id_texture.texture.default_view),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("copy_deferred_lighting_id_pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &deferred_lighting_id_depth_texture.texture.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct CopyDeferredLightingIdPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for CopyDeferredLightingIdPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "copy_deferred_lighting_id_bind_group_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                texture_2d(TextureSampleType::Uint),
            ),
        );

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("copy_deferred_lighting_id_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: COPY_DEFERRED_LIGHTING_ID_SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: Some(DepthStencilState {
                        format: DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Always,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                });

        Self {
            layout,
            pipeline_id,
        }
    }
}

#[derive(Component)]
pub struct DeferredLightingIdDepthTexture {
    pub texture: CachedTexture,
}

fn prepare_deferred_lighting_id_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<DeferredPrepass>>,
) {
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_target_size
        {
            let texture_descriptor = TextureDescriptor {
                label: Some("deferred_lighting_id_depth_texture_a"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                view_formats: &[],
            };
            let texture = texture_cache.get(&render_device, texture_descriptor);
            commands
                .entity(entity)
                .insert(DeferredLightingIdDepthTexture { texture });
        }
    }
}
