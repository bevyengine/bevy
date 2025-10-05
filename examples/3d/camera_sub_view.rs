//! Demonstrates splitting an image across multiple windows with [`SubCameraView`].
//!
//! A `SubCameraView` is a way of cropping the image that a camera renders to its viewport, using a "sheared projection matrix".
//! Some use cases include:
//! - Splitting one image between multiple render targets, as demonstrated by this example
//! - Magnifying a section of the image, as demonstrated by the `magnifier` example
//! - Creating a screen shake effect by rapidly changing the sub view offset
use bevy::{
    camera::{RenderTarget, SubCameraView},
    prelude::*,
    window::WindowRef,
};

const WINDOW_RESOLUTION: (u32, u32) = (640, 360);
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
        transform,
        Camera {
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::ZERO,
            }),
            ..default()
        },
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

    // offset is set to `(0.5, 0.0)` instead of `(1.0, 0.0)` because it controls the top-left corner of the view.
    // As this camera is the top-right quadrant of the overall image, the top-left corner of this quadrant
    // would be halfway along horizontally, and at the very top vertically. Hence the offset being `(0.5, 0.0)`.
    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(top_right)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::new(0.5, 0.0),
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

    // Same logic as the top-right, except for the vertical axis instead of the horizontal axis.
    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(bottom_left)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::new(0.0, 0.5),
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

    // The top-left corner of the bottom-right quadrant is the very middle.
    commands.spawn((
        Camera3d::default(),
        transform,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(bottom_right)),
            sub_camera_view: Some(SubCameraView {
                scale: 0.5,
                offset: Vec2::splat(0.5),
            }),
            ..default()
        },
    ));
}
