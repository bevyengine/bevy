use bevy::{
    prelude::*,
    render::{
        camera::{ActiveCameras, Camera},
        pass::*,
        render_graph::{
            base::MainPass, CameraNode, PassNode, RenderGraph, WindowSwapChainNode,
            WindowTextureNode,
        },
        texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
    },
    window::{CreateWindow, WindowDescriptor, WindowId},
};

/// This example creates a second window and draws a mesh from two different cameras.
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_state(AppState::CreateWindow)
        .add_plugins(DefaultPlugins)
        .add_system_set(SystemSet::on_update(AppState::CreateWindow).with_system(setup_window))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(setup_pipeline))
        .run();
}

// NOTE: this "state based" approach to multiple windows is a short term workaround.
// Future Bevy releases shouldn't require such a strict order of operations.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    CreateWindow,
    Setup,
    Done,
}

fn setup_window(
    mut app_state: ResMut<State<AppState>>,
    mut create_window_events: EventWriter<CreateWindow>,
) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            vsync: false,
            title: "second window".to_string(),
            ..Default::default()
        },
    });

    app_state.set(AppState::Setup).unwrap();
}

fn setup_pipeline(
    mut commands: Commands,
    windows: Res<Windows>,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    asset_server: Res<AssetServer>,
    msaa: Res<Msaa>,
    mut app_state: ResMut<State<AppState>>,
) {
    // get the non-default window id
    let window_id = windows
        .iter()
        .find(|w| w.id() != WindowId::default())
        .map(|w| w.id());

    let window_id = match window_id {
        Some(window_id) => window_id,
        None => return,
    };

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
                sample_count: msaa.samples,
                ..Default::default()
            },
        ),
    );

    // add a new camera node for our new window
    render_graph.add_system_node("secondary_camera", CameraNode::new("Secondary"));

    // add a new render pass for our new window / camera
    let mut second_window_pass = PassNode::<&MainPass>::new(PassDescriptor {
        color_attachments: vec![msaa.color_attachment(
            TextureAttachment::Input("color_attachment".to_string()),
            TextureAttachment::Input("color_resolve_target".to_string()),
            Operations {
                load: LoadOp::Clear(Color::rgb(0.5, 0.5, 0.8)),
                store: true,
            },
        )],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
        sample_count: msaa.samples,
    });

    second_window_pass.add_camera("Secondary");
    active_cameras.add("Secondary");

    render_graph.add_node("second_window_pass", second_window_pass);

    render_graph
        .add_slot_edge(
            "second_window_swap_chain",
            WindowSwapChainNode::OUT_TEXTURE,
            "second_window_pass",
            if msaa.samples > 1 {
                "color_resolve_target"
            } else {
                "color_attachment"
            },
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

    if msaa.samples > 1 {
        render_graph.add_node(
            "second_multi_sampled_color_attachment",
            WindowTextureNode::new(
                window_id,
                TextureDescriptor {
                    size: Extent3d {
                        depth_or_array_layers: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: msaa.samples,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::default(),
                    usage: TextureUsage::OUTPUT_ATTACHMENT,
                },
            ),
        );

        render_graph
            .add_slot_edge(
                "second_multi_sampled_color_attachment",
                WindowSwapChainNode::OUT_TEXTURE,
                "second_window_pass",
                "color_attachment",
            )
            .unwrap();
    }

    // SETUP SCENE

    // add entities to the world
    commands.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..Default::default()
    });
    // main camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    // second window camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        camera: Camera {
            name: Some("Secondary".to_string()),
            window: window_id,
            ..Default::default()
        },
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    app_state.set(AppState::Done).unwrap();
}
