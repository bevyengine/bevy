use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{ActiveCameras, Camera, CameraProjection},
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachment,
            RenderPassDepthStencilAttachment, TextureAttachment,
        },
        render_graph::{
            base::{node::MAIN_PASS, MainPass},
            CameraNode, PassNode, RenderGraph, TextureNode,
        },
        texture::{
            Extent3d, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsage,
        },
    },
    window::WindowId,
};

#[derive(Component)]
pub struct FirstPass;

pub const RENDER_TEXTURE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Texture::TYPE_UUID, 13378939762009864029);

pub const TEXTURE_NODE: &str = "texure_node";
pub const DEPTH_TEXTURE_NODE: &str = "depth_texure_node";
pub const FIRST_PASS: &str = "first_pass";
pub const FIRST_PASS_CAMERA: &str = "first_pass_camera";

fn add_render_to_texture_graph(graph: &mut RenderGraph, size: Extent3d) {
    let mut pass_node = PassNode::<&FirstPass>::new(PassDescriptor {
        color_attachments: vec![RenderPassColorAttachment {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.2, 0.3)),
                store: true,
            },
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
        sample_count: 1,
    });

    pass_node.add_camera(FIRST_PASS_CAMERA);

    graph.add_node(FIRST_PASS, pass_node);
    graph.add_system_node(FIRST_PASS_CAMERA, CameraNode::new(FIRST_PASS_CAMERA));
    graph.add_node_edge(FIRST_PASS_CAMERA, FIRST_PASS).unwrap();

    graph.add_node(
        TEXTURE_NODE,
        TextureNode::new(
            TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: Default::default(),
                usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
            },
            Some(SamplerDescriptor::default()),
            Some(RENDER_TEXTURE_HANDLE),
        ),
    );

    graph.add_node(
        DEPTH_TEXTURE_NODE,
        TextureNode::new(
            TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
            },
            None,
            None,
        ),
    );

    graph.add_node_edge(TEXTURE_NODE, FIRST_PASS).unwrap();
    graph
        .add_slot_edge(
            TEXTURE_NODE,
            TextureNode::TEXTURE,
            FIRST_PASS,
            "color_attachment",
        )
        .unwrap();
    graph
        .add_slot_edge(
            DEPTH_TEXTURE_NODE,
            TextureNode::TEXTURE,
            FIRST_PASS,
            "depth",
        )
        .unwrap();
    graph.add_node_edge(FIRST_PASS, MAIN_PASS).unwrap();
    graph.add_node_edge("transform", FIRST_PASS).unwrap();
}

#[derive(Component)]
struct FirstPassCube;
#[derive(Component)]
struct MainPassCube;

/// rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.5 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_z(1.3 * time.delta_seconds());
    }
}

/// rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.0 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_y(0.7 * time.delta_seconds());
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
    mut active_cameras: ResMut<ActiveCameras>,
) {
    let size = Extent3d::new(512, 512, 1);
    add_render_to_texture_graph(&mut render_graph, size);

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 4.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        roughness: 1.0,
        unlit: false,
        ..Default::default()
    });

    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        })
        .insert(FirstPassCube)
        .insert(FirstPass)
        .remove::<MainPass>();

    // light
    // note: currently lights are shared between passes!
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..Default::default()
    });

    // camera
    let mut first_pass_camera = PerspectiveCameraBundle {
        camera: Camera {
            name: Some(FIRST_PASS_CAMERA.to_string()),
            window: WindowId::new(), /* otherwise it will use main window size / aspect for
                                      * calculation of projection matrix */
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
            .looking_at(Vec3::default(), Vec3::Y),
        ..Default::default()
    };
    active_cameras.add(FIRST_PASS_CAMERA);

    let camera_projection = &mut first_pass_camera.perspective_projection;
    camera_projection.update(size.width as f32, size.height as f32);
    first_pass_camera.camera.projection_matrix = camera_projection.get_projection_matrix();
    first_pass_camera.camera.depth_calculation = camera_projection.depth_calculation();

    commands.spawn_bundle(first_pass_camera);

    let texture_handle = RENDER_TEXTURE_HANDLE.typed();

    let cube_size = 4.0;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle),
        reflectance: 0.02,
        unlit: false,
        ..Default::default()
    });

    // add entities to the world
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.5),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..Default::default()
            },
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(MainPassCube);

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
            .looking_at(Vec3::default(), Vec3::Y),
        ..Default::default()
    });
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(cube_rotator_system)
        .add_system(rotator_system)
        .run();
}
