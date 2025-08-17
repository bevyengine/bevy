use crate::{
    prepass::{DeferredPrepass, ViewPrepassTextures},
    FullscreenShader,
};
use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer};
use bevy_ecs::prelude::*;
use bevy_image::ToExtents;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_resource::{binding_types::texture_2d, *},
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    Render, RenderApp, RenderStartup, RenderSystems,
};

use super::DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT;
use bevy_ecs::query::QueryItem;
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
};
use bevy_utils::default;

pub struct CopyDeferredLightingIdPlugin;

impl Plugin for CopyDeferredLightingIdPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "copy_deferred_lighting_id.wgsl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_copy_deferred_lighting_id_pipeline)
            .add_systems(
                Render,
                (prepare_deferred_lighting_id_textures.in_set(RenderSystems::PrepareResources),),
            );
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

        let diagnostics = render_context.diagnostic_recorder();

        let bind_group = render_context.render_device().create_bind_group(
            "copy_deferred_lighting_id_bind_group",
            &copy_deferred_lighting_id_pipeline.layout,
            &BindGroupEntries::single(&deferred_lighting_pass_id_texture.texture.default_view),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("copy_deferred_lighting_id"),
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

        let pass_span = diagnostics.pass_span(&mut render_pass, "copy_deferred_lighting_id");

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

        Ok(())
    }
}

#[derive(Resource)]
struct CopyDeferredLightingIdPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

pub fn init_copy_deferred_lighting_id_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = render_device.create_bind_group_layout(
        "copy_deferred_lighting_id_bind_group_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::FRAGMENT,
            texture_2d(TextureSampleType::Uint),
        ),
    );

    let vertex_state = fullscreen_shader.to_vertex_state();
    let shader = load_embedded_asset!(asset_server.as_ref(), "copy_deferred_lighting_id.wgsl");

    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("copy_deferred_lighting_id_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            ..default()
        }),
        depth_stencil: Some(DepthStencilState {
            format: DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Always,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        ..default()
    });

    commands.insert_resource(CopyDeferredLightingIdPipeline {
        layout,
        pipeline_id,
    });
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
        if let Some(physical_target_size) = camera.physical_target_size {
            let texture_descriptor = TextureDescriptor {
                label: Some("deferred_lighting_id_depth_texture_a"),
                size: physical_target_size.to_extents(),
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
