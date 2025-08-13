//! Demonstrates different sub view effects.
//!
//! A sub view is essentially a smaller section of a larger viewport. Some use
//! cases include:
//! - Split one image across multiple cameras, for use in a multimonitor setups
//! - Magnify a section of the image, by rendering a small sub view in another
//!   camera
//! - Rapidly change the sub view offset to get a screen shake effect
use bevy::{
    camera::{ScalingMode, SubCameraView, Viewport},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera_view, resize_viewports))
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

    // Main perspective camera:
    //
    // The main perspective image to use as a comparison for the sub views.
    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        ExampleViewports::PerspectiveMain,
        transform,
    ));

    // Perspective camera right half:
    //
    // For this camera, the projection is perspective, and `size` is half the
    // width of the `full_size`, while the x value of `offset` is set to half
    // the value of the full width, causing the right half of the image to be
    // shown. Since the viewport has an aspect ratio of 1x1 and the sub view has
    // an aspect ratio of 1x2, the image appears stretched along the horizontal
    // axis.
    commands.spawn((
        Camera3d::default(),
        Camera {
            sub_camera_view: Some(SubCameraView {
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
        ExampleViewports::PerspectiveStretched,
        transform,
    ));

    // Perspective camera moving:
    //
    // For this camera, the projection is perspective, and the offset is updated
    // continuously in 150 units per second in `move_camera_view`. Since the
    // `full_size` is 500x500, the image should appear to be moving across the
    // full image once every 3.3 seconds. `size` is a fifth of the size of
    // `full_size`, so the image will appear zoomed in.
    commands.spawn((
        Camera3d::default(),
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(500, 500),
                offset: Vec2::ZERO,
                size: UVec2::new(100, 100),
            }),
            order: 2,
            ..default()
        },
        transform,
        ExampleViewports::PerspectiveMoving,
        MovingCameraMarker,
    ));

    // Perspective camera different aspect ratio:
    //
    // For this camera, the projection is perspective, and the aspect ratio of
    // the sub view (2x1) is different to the aspect ratio of the full view
    // (2x2). The aspect ratio of the sub view matches the aspect ratio of
    // the viewport and should show an unstretched image of the top half of the
    // full perspective image.
    commands.spawn((
        Camera3d::default(),
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(800, 800),
                offset: Vec2::ZERO,
                size: UVec2::new(800, 400),
            }),
            order: 3,
            ..default()
        },
        ExampleViewports::PerspectiveControl,
        transform,
    ));

    // Main orthographic camera:
    //
    // The main orthographic image to use as a comparison for the sub views.
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            order: 4,
            ..default()
        },
        ExampleViewports::OrthographicMain,
        transform,
    ));

    // Orthographic camera left half:
    //
    // For this camera, the projection is orthographic, and `size` is half the
    // width of the `full_size`, causing the left half of the image to be shown.
    // Since the viewport has an aspect ratio of 1x1 and the sub view has an
    // aspect ratio of 1x2, the image appears stretched along the horizontal axis.
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(2, 2),
                offset: Vec2::ZERO,
                size: UVec2::new(1, 2),
            }),
            order: 5,
            ..default()
        },
        ExampleViewports::OrthographicStretched,
        transform,
    ));

    // Orthographic camera moving:
    //
    // For this camera, the projection is orthographic, and the offset is
    // updated continuously in 150 units per second in `move_camera_view`. Since
    // the `full_size` is 500x500, the image should appear to be moving across
    // the full image once every 3.3 seconds. `size` is a fifth of the size of
    // `full_size`, so the image will appear zoomed in.
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(500, 500),
                offset: Vec2::ZERO,
                size: UVec2::new(100, 100),
            }),
            order: 6,
            ..default()
        },
        transform,
        ExampleViewports::OrthographicMoving,
        MovingCameraMarker,
    ));

    // Orthographic camera different aspect ratio:
    //
    // For this camera, the projection is orthographic, and the aspect ratio of
    // the sub view (2x1) is different to the aspect ratio of the full view
    // (2x2). The aspect ratio of the sub view matches the aspect ratio of
    // the viewport and should show an unstretched image of the top half of the
    // full orthographic image.
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(200, 200),
                offset: Vec2::ZERO,
                size: UVec2::new(200, 100),
            }),
            order: 7,
            ..default()
        },
        ExampleViewports::OrthographicControl,
        transform,
    ));
}

fn move_camera_view(
    mut movable_camera_query: Query<&mut Camera, With<MovingCameraMarker>>,
    time: Res<Time>,
) {
    for mut camera in movable_camera_query.iter_mut() {
        if let Some(sub_view) = &mut camera.sub_camera_view {
            sub_view.offset.x = (time.elapsed_secs() * 150.) % 450.0 - 50.0;
            sub_view.offset.y = sub_view.offset.x;
        }
    }
}

// To ensure viewports remain the same at any window size
fn resize_viewports(
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut viewports: Query<(&mut Camera, &ExampleViewports)>,
) {
    let window_size = window.physical_size();

    let small_height = window_size.y / 5;
    let small_width = window_size.x / 8;

    let large_height = small_height * 4;
    let large_width = small_width * 4;

    let large_size = UVec2::new(large_width, large_height);

    // Enforce the aspect ratio of the small viewports to ensure the images
    // appear unstretched
    let small_dim = small_height.min(small_width);
    let small_size = UVec2::new(small_dim, small_dim);

    let small_wide_size = UVec2::new(small_dim * 2, small_dim);

    for (mut camera, example_viewport) in viewports.iter_mut() {
        if camera.viewport.is_none() {
            camera.viewport = Some(Viewport::default());
        };

        let Some(viewport) = &mut camera.viewport else {
            continue;
        };

        let (size, position) = match example_viewport {
            ExampleViewports::PerspectiveMain => (large_size, UVec2::new(0, small_height)),
            ExampleViewports::PerspectiveStretched => (small_size, UVec2::ZERO),
            ExampleViewports::PerspectiveMoving => (small_size, UVec2::new(small_width, 0)),
            ExampleViewports::PerspectiveControl => {
                (small_wide_size, UVec2::new(small_width * 2, 0))
            }
            ExampleViewports::OrthographicMain => {
                (large_size, UVec2::new(large_width, small_height))
            }
            ExampleViewports::OrthographicStretched => (small_size, UVec2::new(small_width * 4, 0)),
            ExampleViewports::OrthographicMoving => (small_size, UVec2::new(small_width * 5, 0)),
            ExampleViewports::OrthographicControl => {
                (small_wide_size, UVec2::new(small_width * 6, 0))
            }
        };

        viewport.physical_size = size;
        viewport.physical_position = position;
    }
}

#[derive(Component)]
enum ExampleViewports {
    PerspectiveMain,
    PerspectiveStretched,
    PerspectiveMoving,
    PerspectiveControl,
    OrthographicMain,
    OrthographicStretched,
    OrthographicMoving,
    OrthographicControl,
}
