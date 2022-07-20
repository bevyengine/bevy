//! This example illustrates how to run a winit window in a reactive, low power mode.
//!
//! This is useful for making desktop applications, or any other program that doesn't need to be
//! running the event loop non-stop.

use std::time::Duration;

use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw},
    winit::WinitSettings,
};

fn main() {
    App::new()
        // Continuous rendering for games - bevy's default.
        .insert_resource(WinitSettings::game())
        // Power-saving reactive rendering for applications.
        .insert_resource(WinitSettings::desktop_app())
        // You can also customize update behavior with the fields of [`WinitConfig`]
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_millis(10),
            },
            ..default()
        })
        // Turn off vsync to maximize CPU/GPU usage
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..default()
        })
        .insert_resource(ExampleMode::Game)
        .add_plugins(DefaultPlugins)
        .add_startup_system(test_setup::setup)
        .add_system(test_setup::cycle_modes)
        .add_system(test_setup::rotate_cube)
        .add_system(test_setup::update_text)
        .add_system(update_winit)
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
    mut winit_config: ResMut<WinitSettings>,
) {
    use ExampleMode::*;
    *winit_config = match *mode {
        Game => {
            // In the default `WinitConfig::game()` mode:
            //   * When focused: the event loop runs as fast as possible
            //   * When not focused: the event loop runs as fast as possible
            WinitSettings::game()
        }
        Application => {
            // While in `WinitConfig::desktop_app()` mode:
            //   * When focused: the app will update any time a winit event (e.g. the window is
            //     moved/resized, the mouse moves, a button is pressed, etc.), a [`RequestRedraw`]
            //     event is received, or after 5 seconds if the app has not updated.
            //   * When not focused: the app will update when the window is directly interacted with
            //     (e.g. the mouse hovers over a visible part of the out of focus window), a
            //     [`RequestRedraw`] event is received, or one minute has passed without the app
            //     updating.
            WinitSettings::desktop_app()
        }
        ApplicationWithRedraw => {
            // Sending a `RequestRedraw` event is useful when you want the app to update the next
            // frame regardless of any user input. For example, your application might use
            // `WinitConfig::desktop_app()` to reduce power use, but UI animations need to play even
            // when there are no inputs, so you send redraw requests while the animation is playing.
            event.send(RequestRedraw);
            WinitSettings::desktop_app()
        }
    };
}

/// Everything in this module is for setting up and animating the scene, and is not important to the
/// demonstrated features.
pub(crate) mod test_setup {
    use crate::ExampleMode;
    use bevy::{prelude::*, window::RequestRedraw};

    /// Switch between update modes when the mouse is clicked.
    pub(crate) fn cycle_modes(
        mut mode: ResMut<ExampleMode>,
        mouse_button_input: Res<Input<KeyCode>>,
    ) {
        if mouse_button_input.just_pressed(KeyCode::Space) {
            *mode = match *mode {
                ExampleMode::Game => ExampleMode::Application,
                ExampleMode::Application => ExampleMode::ApplicationWithRedraw,
                ExampleMode::ApplicationWithRedraw => ExampleMode::Game,
            };
        }
    }

    #[derive(Component)]
    pub(crate) struct Rotator;

    /// Rotate the cube to make it clear when the app is updating
    pub(crate) fn rotate_cube(
        time: Res<Time>,
        mut cube_transform: Query<&mut Transform, With<Rotator>>,
    ) {
        for mut transform in cube_transform.iter_mut() {
            transform.rotate_x(time.delta_seconds());
            transform.rotate_local_y(time.delta_seconds());
        }
    }

    #[derive(Component)]
    pub struct ModeText;

    pub(crate) fn update_text(
        mut frame: Local<usize>,
        mode: Res<ExampleMode>,
        mut query: Query<&mut Text, With<ModeText>>,
    ) {
        *frame += 1;
        let mode = match *mode {
            ExampleMode::Game => "game(), continuous, default",
            ExampleMode::Application => "desktop_app(), reactive",
            ExampleMode::ApplicationWithRedraw => "desktop_app(), reactive, RequestRedraw sent",
        };
        query.get_single_mut().unwrap().sections[1].value = mode.to_string();
        query.get_single_mut().unwrap().sections[3].value = format!("{}", *frame);
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
                ..default()
            })
            .insert(Rotator);
        commands.spawn_bundle(PointLightBundle {
            point_light: PointLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        });
        commands.spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        });
        event.send(RequestRedraw);
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    align_self: AlignSelf::FlexStart,
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        top: Val::Px(5.0),
                        left: Val::Px(5.0),
                        ..default()
                    },
                    ..default()
                },
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Press spacebar to cycle modes\n".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::GREEN,
                            },
                        },
                        TextSection {
                            value: "\nFrame: ".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::YELLOW,
                            },
                        },
                        TextSection {
                            value: "".into(),
                            style: TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 50.0,
                                color: Color::YELLOW,
                            },
                        },
                    ],
                    alignment: TextAlignment::default(),
                },
                ..default()
            })
            .insert(ModeText);
    }
}
