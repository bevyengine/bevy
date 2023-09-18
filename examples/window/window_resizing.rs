//! This example illustrates how to resize windows, and how to respond to a window being resized.
use bevy::{prelude::*, window::WindowResized};

fn main() {
    App::new()
        .insert_resource(ResolutionSettings {
            large: Vec2::new(1920.0, 1080.0),
            medium: Vec2::new(800.0, 600.0),
            small: Vec2::new(640.0, 360.0),
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, setup_ui))
        .add_systems(Update, (on_resize_system, toggle_resolution))
        .run();
}

/// Marker component for the text that displays the current resolution.
#[derive(Component)]
struct ResolutionText;

/// Stores the various window-resolutions we can select between.
#[derive(Resource)]
struct ResolutionSettings {
    large: Vec2,
    medium: Vec2,
    small: Vec2,
}

// Spawns the camera that draws UI
fn setup_camera(mut cmd: Commands) {
    cmd.spawn(Camera2dBundle::default());
}

// Spawns the UI
fn setup_ui(mut cmd: Commands) {
    // Node that fills entire background
    cmd.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.),
            ..default()
        },
        ..default()
    })
    .with_children(|root| {
        // Text where we display current resolution
        root.spawn((
            TextBundle::from_section(
                "Resolution",
                TextStyle {
                    font_size: 50.0,
                    color: Color::BLACK,
                    ..default()
                },
            ),
            ResolutionText,
        ));
    });
}

/// This system shows how to request the window to a new resolution
fn toggle_resolution(
    keys: Res<Input<KeyCode>>,
    mut windows: Query<&mut Window>,
    resolution: Res<ResolutionSettings>,
) {
    let mut window = windows.single_mut();

    if keys.just_pressed(KeyCode::Key1) {
        let res = resolution.small;
        window.resolution.set(res.x, res.y);
    }
    if keys.just_pressed(KeyCode::Key2) {
        let res = resolution.medium;
        window.resolution.set(res.x, res.y);
    }
    if keys.just_pressed(KeyCode::Key3) {
        let res = resolution.large;
        window.resolution.set(res.x, res.y);
    }
}

/// This system shows how to respond to a window being resized.
/// Whenever the window is resized, the text will update with the new resolution.
fn on_resize_system(
    mut q: Query<&mut Text, With<ResolutionText>>,
    mut resize_reader: EventReader<WindowResized>,
) {
    let mut text = q.single_mut();
    for e in resize_reader.read() {
        // When resolution is being changed
        text.sections[0].value = format!("{:.1} x {:.1}", e.width, e.height);
    }
}
