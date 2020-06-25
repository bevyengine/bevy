use bevy::{prelude::*, window::CreateWindow};
use bevy_render::{pass::{StoreOp, LoadOp, TextureAttachment, RenderPassColorAttachmentDescriptor, PassDescriptor, RenderPassDepthStencilAttachmentDescriptor}, texture::{TextureDescriptor, TextureFormat, TextureUsage}};
use bevy_window::{WindowId, WindowReference};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(create_second_window_system.system())
        .add_startup_system(setup_scene.system())
        .run();
}

fn create_second_window_system(
    mut create_window_events: ResMut<Events<CreateWindow>>,
    mut render_graph: ResMut<RenderGraph>,
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
        WindowSwapChainNode::new(WindowReference::Id(window_id)),
    );

    // add a new depth texture node for our new window
    render_graph.add_node(
        "second_window_depth_texture",
        WindowTextureNode::new(
            WindowReference::Id(window_id),
            TextureDescriptor {
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::OUTPUT_ATTACHMENT,
                ..Default::default()
            },
        ),
    );

    let mut second_window_pass = PassNode::new(PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color".to_string()),
            resolve_target: None,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_color: Color::rgb(0.1, 0.1, 0.1),
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_load_op: LoadOp::Clear,
            depth_store_op: StoreOp::Store,
            stencil_load_op: LoadOp::Clear,
            stencil_store_op: StoreOp::Store,
            stencil_read_only: false,
            depth_read_only: false,
            clear_depth: 1.0,
            clear_stencil: 0,
        }),
        sample_count: 1,
    });

    // TODO: use different camera here
    second_window_pass.add_camera(bevy::render::base_render_graph::camera::CAMERA);

    render_graph.add_node(
        "second_window_pass",
        second_window_pass,
    );

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
}

fn setup_scene(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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
    command_buffer
        .build()
        // mesh
        .entity_with(MeshComponents {
            mesh: mesh_handle,
            material: material_handle,
            ..Default::default()
        })
        // light
        .entity_with(LightComponents {
            translation: Translation::new(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // main camera
        .entity_with(PerspectiveCameraComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(0.0, 0.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
        // // second window camera
        // .entity_with(PerspectiveCameraComponents {
        //     camera: Camera {
        //         name: Some("Secondary".to_string()),
        //         ..Default::default()
        //     },
        //     transform: Transform::new_sync_disabled(Mat4::face_toward(
        //         Vec3::new(0.0, 0.0, 6.0),
        //         Vec3::new(0.0, 0.0, 0.0),
        //         Vec3::new(0.0, 1.0, 0.0),
        //     )),
        //     ..Default::default()
        // });
}
