//! This example illustrates how to run a winit window in a reactive, low power mode.
//!
//! This is useful for making desktop applications, or any other program that doesn't need to be
//! running the event loop non-stop.

use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw, WindowPlugin},
    winit::{EventLoopProxyWrapper, WakeUp, WinitSettings},
};
use core::time::Duration;

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
    ApplicationWithRequestRedraw,
    ApplicationWithWakeUp,
}

/// Update winit based on the current `ExampleMode`
fn update_winit(
    mode: Res<ExampleMode>,
    mut winit_config: ResMut<WinitSettings>,
    event_loop_proxy: Res<EventLoopProxyWrapper<WakeUp>>,
    mut redraw_request_writer: MessageWriter<RequestRedraw>,
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
            WinitSettings::desktop_app()
        }
        ApplicationWithRequestRedraw => {
            // Sending a `RequestRedraw` event is useful when you want the app to update the next
            // frame regardless of any user input. For example, your application might use
            // `WinitSettings::desktop_app()` to reduce power use, but UI animations need to play even
            // when there are no inputs, so you send redraw requests while the animation is playing.
            // Note that in this example the RequestRedraw winit event will make the app run in the same
            // way as continuous
            redraw_request_writer.write(RequestRedraw);
            WinitSettings::desktop_app()
        }
        ApplicationWithWakeUp => {
            // Sending a `WakeUp` event is useful when you want the app to update the next
            // frame regardless of any user input. This can be used from outside Bevy, see example
            // `window/custom_user_event.rs` for an example usage from outside.
            // Note that in this example the `WakeUp` winit event will make the app run in the same
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

    /// Switch between update modes when the spacebar is pressed.
    pub(crate) fn cycle_modes(
        mut mode: ResMut<ExampleMode>,
        button_input: Res<ButtonInput<KeyCode>>,
    ) {
        if button_input.just_pressed(KeyCode::Space) {
            *mode = match *mode {
                ExampleMode::Game => ExampleMode::Application,
                ExampleMode::Application => ExampleMode::ApplicationWithRequestRedraw,
                ExampleMode::ApplicationWithRequestRedraw => ExampleMode::ApplicationWithWakeUp,
                ExampleMode::ApplicationWithWakeUp => ExampleMode::Game,
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
            transform.rotate_x(time.delta_secs());
            transform.rotate_local_y(time.delta_secs());
        }
    }

    #[derive(Component)]
    pub struct ModeText;

    pub(crate) fn update_text(
        mut frame: Local<usize>,
        mode: Res<ExampleMode>,
        text: Single<Entity, With<ModeText>>,
        mut writer: TextUiWriter,
    ) {
        *frame += 1;
        let mode = match *mode {
            ExampleMode::Game => "game(), continuous, default",
            ExampleMode::Application => "desktop_app(), reactive",
            ExampleMode::ApplicationWithRequestRedraw => {
                "desktop_app(), reactive, RequestRedraw sent"
            }
            ExampleMode::ApplicationWithWakeUp => "desktop_app(), reactive, WakeUp sent",
        };
        *writer.text(*text, 2) = mode.to_string();
        *writer.text(*text, 4) = frame.to_string();
    }

    /// Set up a scene with a cube and some text
    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut request_redraw_writer: MessageWriter<RequestRedraw>,
    ) {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            Rotator,
        ));

        commands.spawn((
            DirectionalLight::default(),
            Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        request_redraw_writer.write(RequestRedraw);
        commands.spawn((
            Text::default(),
            Node {
                align_self: AlignSelf::FlexStart,
                position_type: PositionType::Absolute,
                top: px(12),
                left: px(12),
                ..default()
            },
            ModeText,
            children![
                TextSpan::new("Press space bar to cycle modes\n"),
                (TextSpan::default(), TextColor(LIME.into())),
                (TextSpan::new("\nFrame: "), TextColor(YELLOW.into())),
                (TextSpan::new(""), TextColor(YELLOW.into())),
            ],
        ));
    }
}
