//! Renders multiple cameras with different sub view efffects.
use bevy::{
    math::uvec2,
    prelude::*,
    render::camera::{SubCameraView, Viewport},
};

const PADDING: u32 = 10;
const SMALL_SIZE: u32 = 100;
const LARGE_SIZE: u32 = 450;

const WINDOW_HEIGHT: f32 = (LARGE_SIZE + PADDING * 3 + SMALL_SIZE) as f32;
const WINDOW_WIDTH: f32 = (LARGE_SIZE + PADDING * 2) as f32;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
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
                full_size: uvec2(500, 500),
                offset: uvec2(0, 0),
                size: uvec2(250, 500),
            }),
            order: 1,
            clear_color: ClearColorConfig::None,
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
                    full_size: uvec2(500, 500),
                    offset: uvec2(0, 0),
                    size: uvec2(100, 100),
                }),
                order: 2,
                clear_color: ClearColorConfig::None,
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
                full_size: uvec2(450, 450),
                offset: uvec2(0, 0),
                size: uvec2(450, 450),
            }),
            order: 3,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        transform,
        ..default()
    });

    // Orthographic camera right
    commands.spawn(Camera3dBundle {
        projection: Projection::Orthographic(OrthographicProjection::default),
        camera: Camera {
            viewport: Option::from(Viewport {
                physical_size: uvec2(SMALL_SIZE, SMALL_SIZE),
                physical_position: uvec2(PADDING * 4 + SMALL_SIZE * 3, PADDING),
                ..default()
            }),
            sub_camera_view: Some(SubCameraView {
                full_size: uvec2(500, 500),
                offset: uvec2(250, 0),
                size: uvec2(250, 500),
            }),
            order: 4,
            clear_color: ClearColorConfig::None,
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
    if let Ok(mut camera) = movable_camera_query.get_single_mut() {
        if let Some(sub_view) = &mut camera.sub_camera_view {
            sub_view.offset.x = (time.elapsed_seconds() * 100.) as u32 % 400;
            sub_view.offset.y = sub_view.offset.x;
        }
    }
}
