//! This example illustrates how to run a winit window in a reactive, low power mode.
//!
//! This is useful for making desktop applications, or any other program that doesn't need to be
//! running the event loop non-stop.

use bevy::window::WindowResolution;
use bevy::winit::WakeUp;
use bevy::{
    prelude::*,
    utils::Duration,
    window::{PresentMode, WindowPlugin},
    winit::{EventLoopProxy, WinitSettings},
};

fn main() {
    App::new()
        // Continuous rendering for games - bevy's default.
        .insert_resource(WinitSettings::game())
        // Power-saving reactive rendering for applications.
        .insert_resource(WinitSettings::desktop_app())
        // You can also customize update behavior with the fields of [`WinitSettings`]
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_millis(10)),
        })
        .insert_resource(ExampleMode::Game)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Turn off vsync to maximize CPU/GPU usage
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(800., 640.).with_scale_factor_override(1.),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, test_setup::setup)
        .add_systems(
            Update,
            (
                test_setup::cycle_modes,
                test_setup::rotate_cube,
                test_setup::update_text,
                update_winit,
            ),
        )
        .run();
}

#[derive(Resource, Debug)]
enum ExampleMode {
    Game,
    Application,
    ApplicationWithRedraw,
}

/// Update winit based on the current `ExampleMode`
fn update_winit(
    mode: Res<ExampleMode>,
    mut winit_config: ResMut<WinitSettings>,
    event_loop_proxy: NonSend<EventLoopProxy<WakeUp>>,
) {
    use ExampleMode::*;
    *winit_config = match *mode {
        Game => {
            // In the default `WinitSettings::game()` mode:
            //   * When focused: the event loop runs as fast as possible
            //   * When not focused: the app will update when the window is directly interacted with
            //     (e.g. the mouse hovers over a visible part of the out of focus window), a
            //     [`RequestRedraw`] event is received, or one sixtieth of a second has passed
            //     without the app updating (60 Hz refresh rate max).
            WinitSettings::game()
        }
        Application => {
            // While in `WinitSettings::desktop_app()` mode:
            //   * When focused: the app will update any time a winit event (e.g. the window is
            //     moved/resized, the mouse moves, a button is pressed, etc.), a [`RequestRedraw`]
            //     event is received, or after 5 seconds if the app has not updated.
            //   * When not focused: the app will update when the window is directly interacted with
            //     (e.g. the mouse hovers over a visible part of the out of focus window), a
            //     [`RequestRedraw`] event is received, or one minute has passed without the app
            //     updating.
            WinitSettings {
                focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_secs(1)),
                unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs(5)),
            }
        }
        ApplicationWithRedraw => {
            // Sending a `RequestRedraw` event is useful when you want the app to update the next
            // frame regardless of any user input. For example, your application might use
            // `WinitSettings::desktop_app()` to reduce power use, but UI animations need to play even
            // when there are no inputs, so you send redraw requests while the animation is playing.
            // Note that in this example the RequestRedraw winit event will make the app run in the same
            // way as continuous
            let _ = event_loop_proxy.send_event(WakeUp);
            WinitSettings::desktop_app()
        }
    };
}

/// Everything in this module is for setting up and animating the scene, and is not important to the
/// demonstrated features.
pub(crate) mod test_setup {
    use crate::ExampleMode;
    use bevy::{
        color::palettes::basic::{LIME, YELLOW},
        prelude::*,
        window::RequestRedraw,
    };

    /// Switch between update modes when the mouse is clicked.
    pub(crate) fn cycle_modes(
        mut mode: ResMut<ExampleMode>,
        button_input: Res<ButtonInput<KeyCode>>,
    ) {
        if button_input.just_pressed(KeyCode::Space) {
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
        for mut transform in &mut cube_transform {
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
        let mut text = query.single_mut();
        text.sections[1].value = mode.to_string();
        text.sections[3].value = frame.to_string();
    }

    /// Set up a scene with a cube and some text
    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut event: EventWriter<RequestRedraw>,
    ) {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
                material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
                ..default()
            },
            Rotator,
        ));

        commands.spawn(DirectionalLightBundle {
            transform: Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        });
        commands.spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        });
        event.send(RequestRedraw);
        commands.spawn((
            TextBundle::from_sections([
                TextSection::new(
                    "Press space bar to cycle modes\n",
                    TextStyle {
                        font_size: 50.0,
                        ..default()
                    },
                ),
                TextSection::from_style(TextStyle {
                    font_size: 50.0,
                    color: LIME.into(),
                    ..default()
                }),
                TextSection::new(
                    "\nFrame: ",
                    TextStyle {
                        font_size: 50.0,
                        color: YELLOW.into(),
                        ..default()
                    },
                ),
                TextSection::from_style(TextStyle {
                    font_size: 50.0,
                    color: YELLOW.into(),
                    ..default()
                }),
            ])
            .with_style(Style {
                align_self: AlignSelf::FlexStart,
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            }),
            ModeText,
        ));
    }
}
