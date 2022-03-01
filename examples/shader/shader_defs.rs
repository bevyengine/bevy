use bevy::{
    core_pipeline::Transparent3d,
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::RenderAssets,
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::{
            RenderPipelineCache, RenderPipelineDescriptor, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines,
        },
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};

pub struct IsRedPlugin;

impl Plugin for IsRedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<IsRed>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawIsRed>()
            .init_resource::<IsRedPipeline>()
            .init_resource::<SpecializedMeshPipelines<IsRedPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IsRedPlugin)
        .add_startup_system(setup)
        .run();
}

#[derive(Component, Hash, PartialEq, Eq, Copy, Clone)]
struct IsRed(bool);

impl ExtractComponent for IsRed {
    type Query = &'static IsRed;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        *item
    }
}

/// set up a simple 3D scene
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // red cube
    commands.spawn().insert_bundle((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        IsRed(true),
        Transform::from_xyz(-1.0, 0.5, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));

    // blue cube
    commands.spawn().insert_bundle((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        IsRed(false),
        Transform::from_xyz(1.0, 0.5, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

struct IsRedPipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}

impl FromWorld for IsRedPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>();
        let shader = asset_server.load("shaders/shader_defs.wgsl");
        IsRedPipeline {
            mesh_pipeline: mesh_pipeline.clone(),
            shader,
        }
    }
}

impl SpecializedMeshPipeline for IsRedPipeline {
    type Key = (IsRed, MeshPipelineKey);

    fn specialize(
        &self,
        (is_red, pbr_pipeline_key): Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        if is_red.0 {
            shader_defs.push("IS_RED".to_string());
        }
        let mut descriptor = self.mesh_pipeline.specialize(pbr_pipeline_key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.shader_defs = shader_defs.clone();
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = self.shader.clone();
        fragment.shader_defs = shader_defs;
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);
        Ok(descriptor)
    }
}

type DrawIsRed = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    custom_pipeline: Res<IsRedPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<IsRedPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    material_meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform, &IsRed)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawIsRed>()
        .unwrap();
    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);
    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_handle, mesh_uniform, is_red) in material_meshes.iter() {
            if let Some(mesh) = render_meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(
                        &mut pipeline_cache,
                        &custom_pipeline,
                        (*is_red, key),
                        &mesh.layout,
                    )
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}
