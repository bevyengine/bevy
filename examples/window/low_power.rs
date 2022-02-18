use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw},
    winit::{ControlFlow, WinitConfig},
};

/// This example illustrates how to run a winit window in a reactive, low power mode. This is useful
/// for making desktop applications, or any other program that doesn't need to be running the main
/// loop non-stop.
///
/// * While in `Wait` mode: the app will use almost zero power when the window is minimized,
///   de-focused, or not receiving input.
///
/// * While continuously sending `RequestRedraw` events in `Wait` mode: the app will use resources
///   regardless of window state.
///
/// * While in `Poll` mode: the app will update continuously
fn main() {
    App::new()
        .insert_resource(WinitConfig {
            control_flow: ControlFlow::Wait,
            ..Default::default()
        })
        // Turn off vsync to maximize CPU/GPU usage
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..Default::default()
        })
        .insert_resource(TestMode::Wait)
        .add_plugins(DefaultPlugins)
        .add_startup_system(test_setup::setup)
        .add_system(cycle_modes)
        .add_system(test_setup::rotate)
        .add_system(test_setup::update_text)
        .run();
}

#[derive(Debug)]
enum TestMode {
    Wait,
    WaitAndRedraw,
    Poll,
}

/// Handles switching between update modes
fn cycle_modes(
    mut event: EventWriter<RequestRedraw>,
    mut mode: ResMut<TestMode>,
    mut winit_config: ResMut<WinitConfig>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        *mode = match *mode {
            TestMode::Wait => TestMode::WaitAndRedraw,
            TestMode::WaitAndRedraw => TestMode::Poll,
            TestMode::Poll => TestMode::Wait,
        };
        winit_config.control_flow = match *mode {
            TestMode::Wait => ControlFlow::Wait,
            TestMode::WaitAndRedraw => ControlFlow::Wait,
            TestMode::Poll => ControlFlow::Poll,
        }
    }
    if let TestMode::WaitAndRedraw = *mode {
        // Sending a `RequestRedraw` event is useful when you want the app to update again
        // regardless of any user input. For example, your application might use `ControlFlow::Wait`
        // to reduce power use, but UI animations need to play even when there are no inputs, so you
        // send redraw requests while the animation is playing.
        event.send(RequestRedraw);
    }
}

/// Everything in this module is for setup and is not important to the demonstrated features.
pub(crate) mod test_setup {
    use crate::TestMode;
    use bevy::{prelude::*, window::RequestRedraw};

    #[derive(Component)]
    pub(crate) struct Rotator;

    /// Rotate the cube to make it clear when the app is updating
    pub(crate) fn rotate(mut cube_transform: Query<&mut Transform, With<Rotator>>) {
        for mut transform in cube_transform.iter_mut() {
            transform.rotate(Quat::from_rotation_x(0.04));
            transform.rotate(Quat::from_rotation_y(0.08));
        }
    }

    #[derive(Component)]
    pub struct ModeText;

    pub(crate) fn update_text(mode: Res<TestMode>, mut query: Query<&mut Text, With<ModeText>>) {
        query.get_single_mut().unwrap().sections[1].value = format!("{mode:?}")
    }

    /// Set up a scene with a cube and some text
    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut event: EventWriter<RequestRedraw>,
        asset_server: Res<AssetServer>,
    ) {
        commands
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                ..Default::default()
            })
            .insert(Rotator);
        commands.spawn_bundle(PointLightBundle {
            point_light: PointLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..Default::default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..Default::default()
        });
        commands.spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        });
        event.send(RequestRedraw);
        commands.spawn_bundle(UiCameraBundle::default());
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    align_self: AlignSelf::FlexStart,
                    position_type: PositionType::Absolute,
                    position: Rect {
                        top: Val::Px(5.0),
                        left: Val::Px(5.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Click left mouse button to cycle modes:\n".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "Mode::Wait".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::GREEN,
                            },
                        },
                    ],
                    alignment: TextAlignment::default(),
                },
                ..Default::default()
            })
            .insert(ModeText);
    }
}
