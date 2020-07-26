use bevy::{
    prelude::*,
    render::{
        camera::{ActiveCameras, Camera},
        pass::*,
        render_graph::{CameraNode, PassNode, RenderGraph, WindowSwapChainNode, WindowTextureNode},
        texture::{TextureDescriptor, TextureFormat, TextureUsage},
    },
    window::{CreateWindow, WindowDescriptor, WindowId},
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut create_window_events: ResMut<Events<CreateWindow>>,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800,
            height: 600,
            vsync: false,
            title: "second window".to_string(),
        },
    });

    // here we setup our render graph to draw our second camera to the new window's swap chain

    // add a swapchain node for our new window
    render_graph.add_node(
        "second_window_swap_chain",
        WindowSwapChainNode::new(window_id),
    );

    // add a new depth texture node for our new window
    render_graph.add_node(
        "second_window_depth_texture",
        WindowTextureNode::new(
            window_id,
            TextureDescriptor {
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::OUTPUT_ATTACHMENT,
                ..Default::default()
            },
        ),
    );

    // add a new depth texture node for our new window
    render_graph.add_system_node("secondary_camera", CameraNode::new("Secondary"));

    // add a new render pass for our new camera
    let mut second_window_pass = PassNode::new(PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.1, 0.1)),
                store: true,
            },
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
        sample_count: 1,
    });

    second_window_pass.add_camera("Secondary");
    active_cameras.add("Secondary");

    render_graph.add_node("second_window_pass", second_window_pass);

    render_graph
        .add_slot_edge(
            "second_window_swap_chain",
            WindowSwapChainNode::OUT_TEXTURE,
            "second_window_pass",
            "color",
        )
        .unwrap();

    render_graph
        .add_slot_edge(
            "second_window_depth_texture",
            WindowTextureNode::OUT_TEXTURE,
            "second_window_pass",
            "depth",
        )
        .unwrap();

    render_graph
        .add_node_edge("secondary_camera", "second_window_pass")
        .unwrap();

    // SETUP SCENE

    // load the mesh
    let mesh_handle = asset_server
        .load("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // create a material for the mesh
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // add entities to the world
    commands
        // mesh
        .spawn(PbrComponents {
            mesh: mesh_handle,
            material: material_handle,
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // main camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(0.0, 0.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        // second window camera
        .spawn(Camera3dComponents {
            camera: Camera {
                name: Some("Secondary".to_string()),
                window: window_id,
                ..Default::default()
            },
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(6.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
