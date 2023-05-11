use crate::MeshPipeline;
use crate::{DrawMesh, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::Render;
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
use bevy_utils::tracing::error;

pub const WIREFRAME_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 192598014480025766);

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
            .add_plugin(ExtractResourcePlugin::<WireframeConfig>::default())
            .add_plugin(ExtractComponentPlugin::<Wireframe>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawWireframes>()
                .init_resource::<SpecializedMeshPipelines<WireframePipeline>>()
                .add_systems(Render, queue_wireframes.in_set(RenderSet::Queue));
        }
    }

    fn finish(&self, app: &mut bevy_app::App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<WireframePipeline>();
        }
    }
}

/// Controls whether an entity should rendered in wireframe-mode if the [`WireframePlugin`] is enabled
#[derive(Component, Debug, Clone, Default, ExtractComponent, Reflect)]
#[reflect(Component, Default)]
pub struct Wireframe;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes. If `false`, only meshes with a [Wireframe] component will be rendered.
    pub global: bool,
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
            shader: WIREFRAME_SHADER_HANDLE.typed(),
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
    wireframe_config: Res<WireframeConfig>,
    wireframe_pipeline: Res<WireframePipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WireframePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    mut material_meshes: ParamSet<(
        Query<(Entity, &Handle<Mesh>, &MeshUniform)>,
        Query<(Entity, &Handle<Mesh>, &MeshUniform), With<Wireframe>>,
    )>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = opaque_3d_draw_functions.read().id::<DrawWireframes>();
    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
    for (view, visible_entities, mut opaque_phase) in &mut views {
        let rangefinder = view.rangefinder3d();

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let add_render_phase =
            |(entity, mesh_handle, mesh_uniform): (Entity, &Handle<Mesh>, &MeshUniform)| {
                if let Some(mesh) = render_meshes.get(mesh_handle) {
                    let key = view_key
                        | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                    let pipeline_id = pipelines.specialize(
                        &pipeline_cache,
                        &wireframe_pipeline,
                        key,
                        &mesh.layout,
                    );
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
                        distance: rangefinder.distance(&mesh_uniform.transform),
                    });
                }
            };

        if wireframe_config.global {
            let query = material_meshes.p0();
            visible_entities
                .entities
                .iter()
                .filter_map(|visible_entity| query.get(*visible_entity).ok())
                .for_each(add_render_phase);
        } else {
            let query = material_meshes.p1();
            visible_entities
                .entities
                .iter()
                .filter_map(|visible_entity| query.get(*visible_entity).ok())
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
