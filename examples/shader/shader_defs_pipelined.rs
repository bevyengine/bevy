use bevy::{
    core_pipeline::{SetItemPipeline, Transparent3d},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::prelude::*,
    math::Vec3,
    pbr2::{
        DrawMesh, MeshUniform, PbrPipeline, PbrPipelineKey, SetMeshViewBindGroup,
        SetTransformBindGroup,
    },
    prelude::{App, AssetServer, Assets, GlobalTransform, Handle, Plugin, Transform},
    render2::{
        camera::PerspectiveCameraBundle,
        mesh::{shape, Mesh},
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::*,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
    PipelinedDefaultPlugins,
};

pub struct IsRedPlugin;

impl Plugin for IsRedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<IsRed>::default());
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, DrawIsRed>()
            .init_resource::<IsRedPipeline>()
            .init_resource::<SpecializedPipelines<IsRedPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(IsRedPlugin)
        .add_startup_system(setup)
        .run();
}

#[derive(Hash, PartialEq, Eq, Copy, Clone)]
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
    ));

    // blue cube
    commands.spawn().insert_bundle((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        IsRed(false),
        Transform::from_xyz(1.0, 0.5, 0.0),
        GlobalTransform::default(),
    ));

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

struct IsRedPipeline {
    shader: Handle<Shader>,
    pbr_pipeline: PbrPipeline,
}

impl FromWorld for IsRedPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let pbr_pipeline = world.get_resource::<PbrPipeline>().unwrap();
        let shader = asset_server.load("shaders/shader_defs.wgsl");
        IsRedPipeline {
            shader,
            pbr_pipeline: pbr_pipeline.clone(),
        }
    }
}

impl SpecializedPipeline for IsRedPipeline {
    type Key = IsRed;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.0 {
            shader_defs.push("IS_RED".to_string());
        }
        let mut descriptor = self.pbr_pipeline.specialize(PbrPipelineKey::empty());
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.shader_defs = shader_defs.clone();
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = self.shader.clone();
        fragment.shader_defs = shader_defs;
        descriptor.layout = Some(vec![
            self.pbr_pipeline.view_layout.clone(),
            self.pbr_pipeline.mesh_layout.clone(),
        ]);
        descriptor
    }
}

type DrawIsRed = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTransformBindGroup<1>,
    DrawMesh,
);

fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<IsRedPipeline>,
    mut pipelines: ResMut<SpecializedPipelines<IsRedPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    material_meshes: Query<(Entity, &MeshUniform, &IsRed), With<Handle<Mesh>>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawIsRed>()
        .unwrap();
    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform, is_red) in material_meshes.iter() {
            let pipeline = pipelines.specialize(&mut pipeline_cache, &custom_pipeline, *is_red);
            transparent_phase.add(Transparent3d {
                entity,
                pipeline,
                draw_function: draw_custom,
                distance: view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}
