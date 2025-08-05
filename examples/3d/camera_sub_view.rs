//! Demonstrates different sub view effects.
//!
//! A sub view is essentially a smaller section of a larger viewport. Some use
//! cases include:
//! - Split one image across multiple cameras, for use in a multimonitor setups
//! - Magnify a section of the image, by rendering a small sub view in another
//!   camera
//! - Rapidly change the sub view offset to get a screen shake effect
use bevy::{camera::SubCameraView, prelude::*, window::PrimaryWindow};

#[derive(Resource)]
struct IsSubViewActive(bool);

#[derive(Component)]
struct DebugText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(IsSubViewActive(true))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_sub_view, update_sub_view, update_debug_text),
        )
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let transform = Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Main perspective camera:
    //
    // The main perspective image to use as a comparison for the sub views.
    commands.spawn((Camera3d::default(), Camera::default(), transform));

    // Debug text
    commands
        .spawn((
            Text::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|children| {
            children.spawn(TextSpan::new("Press space to toggle sub view"));
            children.spawn(TextSpan::new("\n"));
            children.spawn((TextSpan::default(), DebugText));
        });
}

fn toggle_sub_view(mut is_active: ResMut<IsSubViewActive>, inputs: Res<ButtonInput<KeyCode>>) {
    if inputs.just_pressed(KeyCode::Space) {
        is_active.0 = !is_active.0;
    }
}

fn update_sub_view(
    mut camera: Single<&mut Camera>,
    window: Single<&Window, With<PrimaryWindow>>,
    is_active: Res<IsSubViewActive>,
) {
    if is_active.0 {
        if let Some(offset) = window.physical_cursor_position() {
            camera.sub_camera_view = Some(SubCameraView {
                full_size: window.physical_size(),
                offset,
                size: window.physical_size() / 2,
            });
        }
    } else {
        camera.sub_camera_view = None;
    }
}

fn update_debug_text(mut text: Single<&mut TextSpan, With<DebugText>>, camera: Single<&Camera>) {
    text.0 = format!("sub_camera_view: {:?}", camera.sub_camera_view);
}
