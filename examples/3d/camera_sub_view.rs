//! Demonstrates different sub view effects.
//!
//! A sub view is essentially a smaller section of a larger viewport. Some use
//! cases include:
//! - Split one image across multiple cameras, for use in a multimonitor setups
//! - Magnify a section of the image, by rendering a small sub view in another
//!   camera
//! - Rapidly change the sub view offset to get a screen shake effect
use bevy::{
    prelude::*,
    render::camera::{ScalingMode, SubCameraView, Viewport},
};

const PADDING: u32 = 10;
const SMALL_SIZE: u32 = 100;
const LARGE_SIZE: u32 = 450;

const WINDOW_HEIGHT: f32 = (LARGE_SIZE + PADDING * 3 + SMALL_SIZE) as f32;
const WINDOW_WIDTH: f32 = (LARGE_SIZE * 2 + PADDING * 3) as f32;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Fix window size to avoid issues with viewports on resizing
                resize_constraints: WindowResizeConstraints {
                    min_width: WINDOW_WIDTH,
                    min_height: WINDOW_HEIGHT,
                    max_width: WINDOW_WIDTH,
                    max_height: WINDOW_HEIGHT,
                },
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, move_camera_view)
        .run();
}

#[derive(Debug, Component)]
struct MovingCameraMarker;

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

    // Main perspective Camera
    commands.spawn(Camera3dBundle {
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(LARGE_SIZE, LARGE_SIZE),
                physical_position: UVec2::new(PADDING, PADDING * 2 + SMALL_SIZE),
                ..default()
            }),
            ..default()
        },
        transform,
        ..default()
    });

    // Perspective camera left half
    commands.spawn(Camera3dBundle {
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                physical_position: UVec2::new(PADDING, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view camera to the right half of the full image
                //
                // The values of `full_size` and `size` do not have to be the
                // exact values of your physical viewport. The important part is
                // the ratio between them.
                full_size: UVec2::new(10, 10),
                // The `offset` is also relative to the values in `full_size`
                // and `size`
                offset: Vec2::new(5.0, 0.0),
                size: UVec2::new(5, 10),
            }),
            order: 1,
            ..default()
        },
        transform,
        ..default()
    });

    // Perspective camera moving
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                viewport: Option::from(Viewport {
                    physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                    physical_position: UVec2::new(PADDING * 2 + SMALL_SIZE, PADDING),
                    ..default()
                }),
                sub_camera_view: Some(SubCameraView {
                    // Set the sub view camera to a fifth of the full view and
                    // move it in another system
                    full_size: UVec2::new(500, 500),
                    offset: Vec2::ZERO,
                    size: UVec2::new(100, 100),
                }),
                order: 2,
                ..default()
            },
            transform,
            ..default()
        },
        MovingCameraMarker,
    ));

    // Perspective camera control
    commands.spawn(Camera3dBundle {
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                physical_position: UVec2::new(PADDING * 3 + SMALL_SIZE * 2, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view to the full image, to ensure that it matches
                // the projection without sub view
                full_size: UVec2::new(450, 450),
                offset: Vec2::ZERO,
                size: UVec2::new(450, 450),
            }),
            order: 3,
            ..default()
        },
        transform,
        ..default()
    });

    // Main orthographic camera
    commands.spawn(Camera3dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(6.0),
            ..OrthographicProjection::default_3d()
        }
        .into(),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(LARGE_SIZE, LARGE_SIZE),
                physical_position: UVec2::new(PADDING * 2 + LARGE_SIZE, PADDING * 2 + SMALL_SIZE),
                ..default()
            }),
            order: 4,
            ..default()
        },
        transform,
        ..default()
    });

    // Orthographic camera left half
    commands.spawn(Camera3dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(6.0),
            ..OrthographicProjection::default_3d()
        }
        .into(),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                physical_position: UVec2::new(PADDING * 5 + SMALL_SIZE * 4, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view camera to the left half of the full image.
                //
                // The values of `full_size` and `size` do not have to be the
                // exact values of your physical viewport. The important part is
                // the ratio between them.
                full_size: UVec2::new(2, 2),
                offset: Vec2::ZERO,
                size: UVec2::new(1, 2),
            }),
            order: 5,
            ..default()
        },
        transform,
        ..default()
    });

    // Orthographic camera moving
    commands.spawn((
        Camera3dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(6.0),
                ..OrthographicProjection::default_3d()
            }
            .into(),
            camera: Camera {
                viewport: Option::from(Viewport {
                    physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                    physical_position: UVec2::new(PADDING * 6 + SMALL_SIZE * 5, PADDING),
                    ..default()
                }),
                sub_camera_view: Some(SubCameraView {
                    // Set the sub view camera to a fifth of the full view and
                    // move it in another system
                    full_size: UVec2::new(500, 500),
                    offset: Vec2::ZERO,
                    size: UVec2::new(100, 100),
                }),
                order: 6,
                ..default()
            },
            transform,
            ..default()
        },
        MovingCameraMarker,
    ));

    // Orthographic camera control
    commands.spawn(Camera3dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(6.0),
            ..OrthographicProjection::default_3d()
        }
        .into(),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: UVec2::new(SMALL_SIZE, SMALL_SIZE),
                physical_position: UVec2::new(PADDING * 7 + SMALL_SIZE * 6, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view to the full image, to ensure that it matches
                // the projection without sub view
                full_size: UVec2::new(450, 450),
                offset: Vec2::ZERO,
                size: UVec2::new(450, 450),
            }),
            order: 7,
            ..default()
        },
        transform,
        ..default()
    });
}

fn move_camera_view(
    mut movable_camera_query: Query<&mut Camera, With<MovingCameraMarker>>,
    time: Res<Time>,
) {
    for mut camera in movable_camera_query.iter_mut() {
        if let Some(sub_view) = &mut camera.sub_camera_view {
            sub_view.offset.x = (time.elapsed_seconds() * 150.) % 450.0 - 50.0;
            sub_view.offset.y = sub_view.offset.x;
        }
    }
}
