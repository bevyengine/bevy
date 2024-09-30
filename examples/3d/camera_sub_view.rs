//! Renders multiple cameras with different sub view efffects.
use bevy::{
    math::{ivec2, uvec2},
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
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::default()),
        material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

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
                physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                physical_position: uvec2(PADDING, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view camera to the left half of the full image
                full_size: uvec2(500, 500),
                offset: ivec2(0, 0),
                size: uvec2(250, 500),
            }),
            order: 1,
            ..default()
        },
        transform,
        ..default()
    });

    // Perpective camera moving
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                viewport: Option::from(Viewport {
                    physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                    physical_position: uvec2(PADDING * 2 + SMALL_SIZE, PADDING),
                    ..default()
                }),
                sub_camera_view: Some(SubCameraView {
                    // Set the sub view camera to a fifth of the full view and
                    // move it in another system
                    full_size: uvec2(500, 500),
                    offset: ivec2(0, 0),
                    size: uvec2(100, 100),
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
                physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                physical_position: uvec2(PADDING * 3 + SMALL_SIZE * 2, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view to the full image, to ensure that it matches
                // the projection without sub view
                full_size: uvec2(450, 450),
                offset: ivec2(0, 0),
                size: uvec2(450, 450),
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
            ..default()
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
            ..default()
        }
        .into(),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                physical_position: uvec2(PADDING * 5 + SMALL_SIZE * 4, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view camera to the left half of the full image
                full_size: uvec2(500, 500),
                offset: ivec2(0, 0),
                size: uvec2(250, 500),
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
                ..default()
            }
            .into(),
            camera: Camera {
                viewport: Option::from(Viewport {
                    physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                    physical_position: uvec2(PADDING * 6 + SMALL_SIZE * 5, PADDING),
                    ..default()
                }),
                sub_camera_view: Some(SubCameraView {
                    // Set the sub view camera to a fifth of the full view and
                    // move it in another system
                    full_size: uvec2(500, 500),
                    offset: ivec2(0, 0),
                    size: uvec2(100, 100),
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
            ..default()
        }
        .into(),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                physical_position: uvec2(PADDING * 7 + SMALL_SIZE * 6, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                // Set the sub view to the full image, to ensure that it matches
                // the projection without sub view
                full_size: uvec2(450, 450),
                offset: ivec2(0, 0),
                size: uvec2(450, 450),
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
            sub_view.offset.x = (time.elapsed_seconds() * 150.) as i32 % 450 - 50;
            sub_view.offset.y = sub_view.offset.x;
        }
    }
}
