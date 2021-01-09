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
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_resource(State::new(AppState::CreateWindow))
        .add_plugins(DefaultPlugins)
        .add_stage_after(
            stage::UPDATE,
            STATE_STAGE,
            StateStage::<AppState>::default(),
        )
        .on_state_update(STATE_STAGE, AppState::CreateWindow, setup_window.system())
        .on_state_enter(STATE_STAGE, AppState::Setup, setup_pipeline.system())
        .run();
}

const STATE_STAGE: &str = "state";

// NOTE: this "state based" approach to multiple windows is a short term workaround.
// Future Bevy releases shouldn't require such a strict order of operations.
#[derive(Clone)]
enum AppState {
    CreateWindow,
    Setup,
}

fn setup_window(
    mut app_state: ResMut<State<AppState>>,
    mut create_window_events: ResMut<Events<CreateWindow>>,
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

    app_state.set_next(AppState::Setup).unwrap();
}

fn setup_pipeline(
    commands: &mut Commands,
    windows: Res<Windows>,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    asset_server: Res<AssetServer>,
    msaa: Res<Msaa>,
) {
    // get the non-default window id
    let window_id = windows
        .iter()
        .find(|w| w.id() != WindowId::default())
        .map(|w| w.id())
        .unwrap();

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
        color_attachments: vec![msaa.color_attachment_descriptor(
            TextureAttachment::Input("color_attachment".to_string()),
            TextureAttachment::Input("color_resolve_target".to_string()),
            Operations {
                load: LoadOp::Clear(Color::rgb(0.5, 0.5, 0.8)),
                store: true,
            },
        )],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
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
                        depth: 1,
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
    commands
        .spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"))
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // main camera
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 6.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        })
        // second window camera
        .spawn(Camera3dBundle {
            camera: Camera {
                name: Some("Secondary".to_string()),
                window: window_id,
                ..Default::default()
            },
            transform: Transform::from_xyz(6.0, 0.0, 0.0)
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
