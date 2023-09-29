use crate::MeshPipeline;
use crate::{
    DrawMesh, MeshPipelineKey, RenderMeshInstance, RenderMeshInstances, SetMeshBindGroup,
    SetMeshViewBindGroup,
};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::{
        PipelineCache, PolygonMode, RenderPipelineDescriptor, Shader, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    view::{ExtractedView, Msaa, VisibleEntities},
    RenderApp, RenderSet,
};
use bevy_render::{Extract, ExtractSchedule, Render};
use bevy_utils::tracing::error;
use bevy_utils::EntityHashSet;

pub const WIREFRAME_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(192598014480025766);

#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            WIREFRAME_SHADER_HANDLE,
            "render/wireframe.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Wireframe>()
            .register_type::<WireframeConfig>()
            .init_resource::<WireframeConfig>()
            .add_plugins((ExtractResourcePlugin::<WireframeConfig>::default(),));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawWireframes>()
                .init_resource::<SpecializedMeshPipelines<WireframePipeline>>()
                .init_resource::<Wireframes>()
                .add_systems(ExtractSchedule, extract_wireframes)
                .add_systems(Render, queue_wireframes.in_set(RenderSet::QueueMeshes));
        }
    }

    fn finish(&self, app: &mut bevy_app::App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<WireframePipeline>();
        }
    }
}

/// Controls whether an entity should rendered in wireframe-mode if the [`WireframePlugin`] is enabled
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Wireframe;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes. If `false`, only meshes with a [`Wireframe`] component will be rendered.
    pub global: bool,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Wireframes(EntityHashSet<Entity>);

fn extract_wireframes(
    mut wireframes: ResMut<Wireframes>,
    query: Extract<Query<Entity, With<Wireframe>>>,
) {
    wireframes.clear();
    wireframes.extend(&query);
}

#[derive(Resource, Clone)]
pub struct WireframePipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}
impl FromWorld for WireframePipeline {
    fn from_world(render_world: &mut World) -> Self {
        WireframePipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            shader: WIREFRAME_SHADER_HANDLE,
        }
    }
}

impl SpecializedMeshPipeline for WireframePipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone_weak();
        descriptor
            .vertex
            .shader_defs
            .push("MESH_BINDGROUP_1".into());
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone_weak();
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_wireframes(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    wireframes: Res<Wireframes>,
    wireframe_config: Res<WireframeConfig>,
    wireframe_pipeline: Res<WireframePipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WireframePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = opaque_3d_draw_functions.read().id::<DrawWireframes>();
    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
    for (view, visible_entities, mut opaque_phase) in &mut views {
        let rangefinder = view.rangefinder3d();

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let add_render_phase = |phase_item: (Entity, &RenderMeshInstance)| {
            let (entity, mesh_instance) = phase_item;

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                return;
            };
            let mut key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
            if mesh.morph_targets.is_some() {
                key |= MeshPipelineKey::MORPH_TARGETS;
            }
            let pipeline_id =
                pipelines.specialize(&pipeline_cache, &wireframe_pipeline, key, &mesh.layout);
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    return;
                }
            };
            opaque_phase.add(Opaque3d {
                entity,
                pipeline: pipeline_id,
                draw_function: draw_custom,
                distance: rangefinder
                    .distance_translation(&mesh_instance.transforms.transform.translation),
                batch_range: 0..1,
                dynamic_offset: None,
            });
        };

        if wireframe_config.global {
            visible_entities
                .entities
                .iter()
                .filter_map(|visible_entity| {
                    render_mesh_instances
                        .get(visible_entity)
                        .map(|mesh_instance| (*visible_entity, mesh_instance))
                })
                .for_each(add_render_phase);
        } else {
            visible_entities
                .entities
                .iter()
                .filter_map(|visible_entity| {
                    if wireframes.contains(visible_entity) {
                        render_mesh_instances
                            .get(visible_entity)
                            .map(|mesh_instance| (*visible_entity, mesh_instance))
                    } else {
                        None
                    }
                })
                .for_each(add_render_phase);
        }
    }
}

type DrawWireframes = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);
