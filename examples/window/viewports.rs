use bevy::{
    math::{clamp, Rect},
    prelude::*,
    render::{
        camera::{ActiveCameras, Camera},
        render_graph::{base, CameraNode, PassNode, RenderGraph},
        surface::{SideLocation, Viewport, ViewportDescriptor},
    },
};

/// This example creates a second window and draws a mesh from two different cameras.
fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .init_resource::<ViewportLayout>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(viewport_layout_system.system())
        .run();
}

const FRONT_CAMERA: &str = "FrontView";
const FRONT_CAMERA_NODE: &str = "front_view_camera";
const SIDE_CAMERA: &str = "SideView";
const SIDE_CAMERA_NODE: &str = "side_view_camera";

fn setup(
    commands: &mut Commands,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    asset_server: Res<AssetServer>,
) {
    // add new camera nodes for the secondary viewports
    render_graph.add_system_node(FRONT_CAMERA_NODE, CameraNode::new(FRONT_CAMERA));
    render_graph.add_system_node(SIDE_CAMERA_NODE, CameraNode::new(SIDE_CAMERA));
    active_cameras.add(FRONT_CAMERA);
    active_cameras.add(SIDE_CAMERA);

    // add the cameras to the main pass
    {
        let main_pass: &mut PassNode<&base::MainPass> =
            render_graph.get_node_mut(base::node::MAIN_PASS).unwrap();
        main_pass.add_camera(FRONT_CAMERA);
        main_pass.add_camera(SIDE_CAMERA);
    }
    render_graph
        .add_node_edge(FRONT_CAMERA_NODE, base::node::MAIN_PASS)
        .unwrap();
    render_graph
        .add_node_edge(SIDE_CAMERA_NODE, base::node::MAIN_PASS)
        .unwrap();

    // SETUP SCENE

    // add entities to the world
    commands
        //.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"))
        .spawn_scene(asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"))
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // main camera
        .spawn(PerspectiveCameraBundle {
            // the following is an example of how to setup static viewports
            // and isn't really necessary in this case, as it will be
            // immediately overwritten by the viewport_layout_system
            viewport: Viewport::new(ViewportDescriptor {
                sides: Rect {
                    // occupy the left 50% of the available horizontal space
                    left: SideLocation::Relative(0.0),
                    right: SideLocation::Relative(0.5),
                    // occupy the left 100% of the available vertical space
                    top: SideLocation::Relative(0.0),
                    bottom: SideLocation::Relative(1.0),
                },
                ..Default::default()
            }),
            transform: Transform::from_xyz(-1.0, 1.0, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::unit_y()),
            ..Default::default()
        })
        // top right camera
        .spawn(PerspectiveCameraBundle {
            camera: Camera {
                name: Some(FRONT_CAMERA.to_string()),
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.3, 1.3)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::unit_y()),
            ..Default::default()
        })
        // bottom right camera
        .spawn(PerspectiveCameraBundle {
            camera: Camera {
                name: Some(SIDE_CAMERA.to_string()),
                ..Default::default()
            },
            transform: Transform::from_xyz(-1.3, 0.3, 0.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::unit_y()),
            ..Default::default()
        });

    // ui
    let instructions_text =
        "Use the arrow keys to resize the viewports\nPress Enter to swap the rightmost viewports";
    commands
        .spawn(UiCameraBundle {
            // viewports occupy the entire surface by default, and can overlap each other
            ..Default::default()
        })
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text::with_section(
                instructions_text,
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
                Default::default(),
            ),
            ..Default::default()
        });
}

struct ViewportLayout {
    divide_x: f32,
    divide_y: f32,
    invert: bool,
}

impl ViewportLayout {
    pub fn main_view(&self) -> Rect<SideLocation> {
        Rect {
            left: SideLocation::Relative(0.0),
            right: SideLocation::Relative(self.divide_x),
            top: SideLocation::Relative(0.0),
            bottom: SideLocation::Relative(1.0),
        }
    }

    pub fn front_view_view(&self) -> Rect<SideLocation> {
        Rect {
            left: SideLocation::Relative(self.divide_x),
            right: SideLocation::Relative(1.0),
            top: SideLocation::Relative(0.0),
            bottom: SideLocation::Relative(self.divide_y),
        }
    }

    pub fn side_view_view(&self) -> Rect<SideLocation> {
        Rect {
            left: SideLocation::Relative(self.divide_x),
            right: SideLocation::Relative(1.0),
            top: SideLocation::Relative(self.divide_y),
            bottom: SideLocation::Relative(1.0),
        }
    }
}

impl Default for ViewportLayout {
    fn default() -> Self {
        Self {
            divide_x: 0.5,
            divide_y: 0.5,
            invert: false,
        }
    }
}

fn viewport_layout_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut layout: ResMut<ViewportLayout>,
    mut query: Query<(&Camera, &mut Viewport)>,
) {
    // update the layout state
    if keyboard_input.just_pressed(KeyCode::Left) {
        layout.divide_x -= 0.05;
    }
    if keyboard_input.just_pressed(KeyCode::Right) {
        layout.divide_x += 0.05;
    }
    if keyboard_input.just_pressed(KeyCode::Up) {
        layout.divide_y -= 0.05;
    }
    if keyboard_input.just_pressed(KeyCode::Down) {
        layout.divide_y += 0.05;
    }
    if keyboard_input.just_pressed(KeyCode::Return) {
        layout.invert = !layout.invert;
    }
    layout.divide_x = clamp(layout.divide_x, 0.0, 1.0);
    layout.divide_y = clamp(layout.divide_y, 0.0, 1.0);

    // resize the viewports
    for (camera, mut viewport) in query.iter_mut() {
        match camera.name.as_deref() {
            // default camera
            Some("Camera3d") => {
                viewport.sides = layout.main_view();
            }
            Some(FRONT_CAMERA) => {
                if layout.invert {
                    viewport.sides = layout.front_view_view();
                } else {
                    viewport.sides = layout.side_view_view();
                }
            }
            Some(SIDE_CAMERA) => {
                if layout.invert {
                    viewport.sides = layout.side_view_view();
                } else {
                    viewport.sides = layout.front_view_view();
                }
            }
            _ => {}
        }
    }
}
