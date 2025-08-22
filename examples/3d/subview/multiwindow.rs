//! Demonstrates splitting an image across multiple windows with SubCameraView
use bevy::{camera::RenderTarget, prelude::*, render::camera::SubCameraView, window::WindowRef};

const WINDOW_RESOLUTION: (u16, u16) = (640, 360);
const WINDOW_POS_OFFSET: IVec2 = IVec2::new(50, 50);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Top Left".into(),
                resolution: WINDOW_RESOLUTION.into(),
                position: WindowPosition::new(WINDOW_POS_OFFSET),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    let transform = Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Camera for the primary window (Top Left)
    commands.spawn((
        Camera3d::default(),
        Camera {
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::ZERO,
                ..default()
            }),
            ..default()
        },
        transform,
    ));

    let top_right = commands
        .spawn(Window {
            title: "Top Right".to_owned(),
            resolution: WINDOW_RESOLUTION.into(),
            position: WindowPosition::new(
                WINDOW_POS_OFFSET + IVec2::new(WINDOW_RESOLUTION.0 as _, 0),
            ),
            ..default()
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(top_right)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::new(0.5, 0.0),
                ..default()
            }),
            ..default()
        },
    ));

    let bottom_left = commands
        .spawn(Window {
            title: "Bottom Left".to_owned(),
            resolution: WINDOW_RESOLUTION.into(),
            position: WindowPosition::new(
                WINDOW_POS_OFFSET + IVec2::new(0, WINDOW_RESOLUTION.1 as _),
            ),
            ..default()
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(bottom_left)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::new(0.0, 0.5),
                ..default()
            }),
            ..default()
        },
    ));

    let bottom_right = commands
        .spawn(Window {
            title: "Bottom Right".to_owned(),
            resolution: WINDOW_RESOLUTION.into(),
            position: WindowPosition::new(
                WINDOW_POS_OFFSET + IVec2::new(WINDOW_RESOLUTION.0 as _, WINDOW_RESOLUTION.1 as _),
            ),
            ..default()
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(bottom_right)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::splat(0.5),
                ..default()
            }),
            ..default()
        },
    ));
}
