use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw},
    winit::{ControlFlow, WinitConfig},
};

/// This example illustrates how to run a winit window in a reactive, low power mode, useful for
/// making desktop applications or any other program that doesn't need to be running the main loop
/// non-stop. The app will only update when there is an event (resize, mouse input, etc.), or a
/// redraw request is sent to force an update.
///
/// When the window is minimized, de-focused, or not receiving input, it will use almost zero power.
fn main() {
    App::new()
        // Note: you can change the control_flow setting while the app is running
        .insert_resource(WinitConfig {
            control_flow: ControlFlow::Wait,
            ..Default::default()
        })
        // Turn off vsync to use maximum CPU/GPU when running all-out
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..Default::default()
        })
        .insert_resource(ManuallyRedraw(false))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(toggle_mode)
        .add_system(rotate)
        .run();
}

struct ManuallyRedraw(bool);

/// This system runs every update and switches between two modes when the left mouse button is
/// clicked:
/// 1) Continuously sending redraw requests
/// 2) Only updating the app when an input is received
fn toggle_mode(
    mut event: EventWriter<RequestRedraw>,
    mut redraw: ResMut<ManuallyRedraw>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        redraw.0 = !redraw.0;
    }
    if redraw.0 {
        event.send(RequestRedraw);
    }
}

fn rotate(mut cube_transform: Query<&mut Transform, With<Rotator>>) {
    for mut transform in cube_transform.iter_mut() {
        transform.rotate(Quat::from_rotation_x(0.05));
        transform.rotate(Quat::from_rotation_y(0.08));
    }
}

#[derive(Component)]
struct Rotator;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event: EventWriter<RequestRedraw>,
    asset_server: Res<AssetServer>,
) {
    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            ..Default::default()
        })
        .insert(Rotator);
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    event.send(RequestRedraw);
    // UI camera
    commands.spawn_bundle(UiCameraBundle::default());
    // Text with one section
    commands.spawn_bundle(TextBundle {
        style: Style {
            align_self: AlignSelf::FlexEnd,
            position_type: PositionType::Absolute,
            position: Rect {
                bottom: Val::Px(5.0),
                right: Val::Px(15.0),
                ..Default::default()
            },
            ..Default::default()
        },
        // Use the `Text::with_section` constructor
        text: Text::with_section(
            "Click left mouse button to toggle continuous redraw requests",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 50.0,
                color: Color::WHITE,
            },
            TextAlignment::default(),
        ),
        ..Default::default()
    });
}
