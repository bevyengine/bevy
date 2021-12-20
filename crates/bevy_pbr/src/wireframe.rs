use crate::MeshPipeline;
use crate::{DrawMesh, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup};
use bevy_app::Plugin;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_core_pipeline::Opaque3d;
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    mesh::Mesh,
    render_asset::RenderAssets,
    render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::{RenderPipelineCache, Shader, SpecializedPipeline, SpecializedPipelines},
    view::{ExtractedView, Msaa},
    RenderApp, RenderStage,
};
use wgpu::PolygonMode;

pub const WIREFRAME_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 192598014480025766);

#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            WIREFRAME_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("render/wireframe.wgsl")),
        );

        app.init_resource::<WireframeConfig>();

        app.sub_app(RenderApp)
            .add_render_command::<Opaque3d, DrawWireframes>()
            .init_resource::<WireframePipeline>()
            .init_resource::<SpecializedPipelines<WireframePipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_wireframes)
            .add_system_to_stage(RenderStage::Extract, extract_wireframe_config)
            .add_system_to_stage(RenderStage::Queue, queue_wireframes);
    }
}

fn extract_wireframe_config(mut commands: Commands, wireframe_config: Res<WireframeConfig>) {
    if wireframe_config.is_added() || wireframe_config.is_changed() {
        commands.insert_resource(wireframe_config.into_inner().clone());
    }
}

fn extract_wireframes(mut commands: Commands, query: Query<Entity, With<Wireframe>>) {
    for entity in query.iter() {
        commands.get_or_spawn(entity).insert(Wireframe);
    }
}

/// Controls whether an entity should rendered in wireframe-mode if the [WireframePlugin] is enabled
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct Wireframe;

#[derive(Debug, Clone, Default)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes. If `false`, only meshes with a [Wireframe] component will be rendered.
    pub global: bool,
}

pub struct WireframePipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}
impl FromWorld for WireframePipeline {
    fn from_world(render_world: &mut World) -> Self {
        WireframePipeline {
            mesh_pipeline: render_world.get_resource::<MeshPipeline>().unwrap().clone(),
            shader: WIREFRAME_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedPipeline for WireframePipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> bevy_render::render_resource::RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone_weak();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone_weak();
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        descriptor
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_wireframes(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    wireframe_config: Res<WireframeConfig>,
    wireframe_pipeline: Res<WireframePipeline>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedPipelines<WireframePipeline>>,
    msaa: Res<Msaa>,
    mut material_meshes: QuerySet<(
        QueryState<(Entity, &Handle<Mesh>, &MeshUniform)>,
        QueryState<(Entity, &Handle<Mesh>, &MeshUniform), With<Wireframe>>,
    )>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = opaque_3d_draw_functions
        .read()
        .get_id::<DrawWireframes>()
        .unwrap();
    let key = MeshPipelineKey::from_msaa_samples(msaa.samples);
    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);

        let add_render_phase =
            |(entity, mesh_handle, mesh_uniform): (Entity, &Handle<Mesh>, &MeshUniform)| {
                if let Some(mesh) = render_meshes.get(mesh_handle) {
                    let key =
                        key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                    transparent_phase.add(Opaque3d {
                        entity,
                        pipeline: specialized_pipelines.specialize(
                            &mut pipeline_cache,
                            &wireframe_pipeline,
                            key,
                        ),
                        draw_function: draw_custom,
                        distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                    });
                }
            };

        if wireframe_config.global {
            material_meshes.q0().iter().for_each(add_render_phase);
        } else {
            material_meshes.q1().iter().for_each(add_render_phase);
        }
    }
}

type DrawWireframes = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);
