use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw},
    winit::WinitConfig,
};

/// This example illustrates how to run a winit window in a reactive, low power mode. This is useful
/// for making desktop applications, or any other program that doesn't need to be running the event
/// loop non-stop.
///
/// * In the default `WinitConfig::game()` mode, the event loop runs as fast as possible when the
///   window is focused. When not in focus, the app updates every 100ms.
///
/// * While in [`bevy::winit::WinitConfig::desktop_app()`] mode:
///     * When focused: the app will update any time a winit event (e.g. the window is
///       moved/resized, the mouse moves, a button is pressed, etc.) or [`RequestRedraw`] event is
///       received, or after 5 seconds if the app has not updated.
///     * When not focused: the app will update when a [`RequestRedraw`] event is received, the
///       window is directly interacted with (e.g. the mouse hovers over a visible part of the out
///       of focus window), or one minute has passed without the app updating.
///
/// These two functions are presets, you can customize the behavior by manually setting the fields
/// of the [`WinitConfig`] resource.
fn main() {
    App::new()
        .insert_resource(WinitConfig::game())
        // Turn off vsync to maximize CPU/GPU usage
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..Default::default()
        })
        .insert_resource(ExampleMode::Game)
        .add_plugins(DefaultPlugins)
        .add_startup_system(test_setup::setup)
        .add_system(cycle_modes)
        .add_system(update_winit)
        .add_system(test_setup::rotate)
        .add_system(test_setup::update_text)
        .run();
}

#[derive(Debug)]
enum ExampleMode {
    Game,
    Application,
    ApplicationWithRedraw,
}

/// Update winit based on the current `ExampleMode`
fn update_winit(
    mode: Res<ExampleMode>,
    mut event: EventWriter<RequestRedraw>,
    mut winit_config: ResMut<WinitConfig>,
) {
    *winit_config = match *mode {
        ExampleMode::Game => WinitConfig::game(),
        ExampleMode::Application => WinitConfig::desktop_app(),
        ExampleMode::ApplicationWithRedraw => WinitConfig::desktop_app(),
    };

    if let ExampleMode::ApplicationWithRedraw = *mode {
        // Sending a `RequestRedraw` event is useful when you want the app to update again
        // regardless of any user input. For example, your application might use
        // `WinitConfig::desktop_app()` to reduce power use, but UI animations need to play even
        // when there are no inputs, so you send redraw requests while the animation is playing.
        event.send(RequestRedraw);
    }
}

/// Switch between update modes when the mouse is clicked.
fn cycle_modes(mut mode: ResMut<ExampleMode>, mouse_button_input: Res<Input<MouseButton>>) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        *mode = match *mode {
            ExampleMode::Game => ExampleMode::Application,
            ExampleMode::Application => ExampleMode::ApplicationWithRedraw,
            ExampleMode::ApplicationWithRedraw => ExampleMode::Game,
        };
    }
}

/// Everything in this module is for setting up and animating the scene, and is not important to the
/// demonstrated features.
pub(crate) mod test_setup {
    use crate::ExampleMode;
    use bevy::{prelude::*, window::RequestRedraw};

    #[derive(Component)]
    pub(crate) struct Rotator;

    /// Rotate the cube to make it clear when the app is updating
    pub(crate) fn rotate(
        time: Res<Time>,
        mut cube_transform: Query<&mut Transform, With<Rotator>>,
    ) {
        for mut transform in cube_transform.iter_mut() {
            let t = time.seconds_since_startup() as f32;
            *transform =
                transform.with_rotation(Quat::from_rotation_x(t) * Quat::from_rotation_y(t));
        }
    }

    #[derive(Component)]
    pub struct ModeText;

    pub(crate) fn update_text(mode: Res<ExampleMode>, mut query: Query<&mut Text, With<ModeText>>) {
        query.get_single_mut().unwrap().sections[1].value = format!("{:?}", *mode)
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
