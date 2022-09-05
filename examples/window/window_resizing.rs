///! This example illustrates how to resize windows, and how to respond to a window being resized.
use bevy::{prelude::*, window::WindowResized};

fn main() {
    App::new()
        .insert_resource(ResolutionSettings {
            large: Vec2::new(1920.0, 1080.0),
            medium: Vec2::new(800.0, 600.0),
            small: Vec2::new(640.0, 360.0),
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_ui)
        .add_system(on_resize_system)
        .add_system(toggle_resolution)
        .run();
}

/// Marker component for the text that displays the current reslution.
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
    cmd.spawn_bundle(Camera2dBundle::default());
}

// Spawns the UI
fn setup_ui(mut cmd: Commands, asset_server: Res<AssetServer>) {
    // Node that fills entire background
    cmd.spawn_bundle(NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            ..default()
        },
        ..default()
    })
    .with_children(|root| {
        // Text where we display current resolution
        root.spawn_bundle(TextBundle::from_section(
            "Resolution",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 50.0,
                color: Color::BLACK,
            },
        ))
        .insert(ResolutionText);
    });
}

/// This system shows how to request the window to a new resolution
fn toggle_resolution(
    keys: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    resolution: Res<ResolutionSettings>,
) {
    let window = windows.get_primary_mut().unwrap();

    if keys.just_pressed(KeyCode::Key1) {
        let res = resolution.small;
        window.set_resolution(res.x, res.y);
    }
    if keys.just_pressed(KeyCode::Key2) {
        let res = resolution.medium;
        window.set_resolution(res.x, res.y);
    }
    if keys.just_pressed(KeyCode::Key3) {
        let res = resolution.large;
        window.set_resolution(res.x, res.y);
    }
}

/// This system shows how to respond to a window being resized.
/// Whenever the window is resized, the text will update with the new resolution.
fn on_resize_system(
    mut q: Query<&mut Text, With<ResolutionText>>,
    mut resize_reader: EventReader<WindowResized>,
) {
    let mut text = q.get_single_mut().unwrap();
    for e in resize_reader.iter() {
        // When resolution is being changed
        text.sections[0].value = format!("{:.1} x {:.1}", e.width, e.height);
    }
}
