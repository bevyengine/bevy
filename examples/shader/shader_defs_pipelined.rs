use bevy::{
    core_pipeline::{SetItemPipeline, Transparent3d},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::prelude::*,
    math::Vec3,
    pbr2::{DrawMesh, MeshUniform, PbrPipeline, SetMeshViewBindGroup, SetTransformBindGroup},
    prelude::{App, AssetServer, Assets, GlobalTransform, Handle, Plugin, Transform},
    render2::{
        camera::PerspectiveCameraBundle,
        mesh::{shape, Mesh},
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase},
        render_resource::*,
        texture::BevyDefault,
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
    view_layout: BindGroupLayout,
    mesh_layout: BindGroupLayout,
}

impl FromWorld for IsRedPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let pbr_pipeline = world.get_resource::<PbrPipeline>().unwrap();
        let shader = asset_server.load("shaders/shader_defs.wgsl");
        IsRedPipeline {
            shader,
            view_layout: pbr_pipeline.view_layout.clone(),
            mesh_layout: pbr_pipeline.mesh_layout.clone(),
        }
    }
}

impl SpecializedPipeline for IsRedPipeline {
    type Key = IsRed;

    fn specialize(&self, key: Self::Key) -> OwnedRenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.0 {
            shader_defs.push("IS_RED".to_string());
        }
        OwnedRenderPipelineDescriptor {
            vertex: OwnedVertexState {
                shader: self.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![OwnedVertexBufferLayout {
                    array_stride: 32,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 0,
                        },
                        // Normal
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            fragment: Some(OwnedFragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![self.view_layout.clone(), self.mesh_layout.clone()]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: None,
        }
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
