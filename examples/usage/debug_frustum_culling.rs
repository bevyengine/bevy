//! This example demonstrates how to debug and visualize frustum culling,
//! a process (and Bevy [`system`](bevy::camera::visibility::check_visibility)) that determines
//! which entities are visible and should be rendered within
//! a camera's view. If an entity's [`Aabb`](bevy::camera::primitives::Aabb)
//! (Axis-Aligned Bounding Box) does not intersect with a camera's
//! [`Frustum`](bevy::camera::primitives::Frustum), that entity is said to be "culled"
//! from the camera's view frustum.
//!
//! To debug and visualize frustum culling, this example uses Aabb and Frustum gizmos provided
//! by bevy's [`Gizmo`] library. [`Aabb gizmos`](bevy::gizmos::aabb) are used to visualize the
//! [`Aabb`](bevy::camera::primitives::Aabb) of entities.
//! [`Frustum gizmos`](bevy::gizmos::frustum) are used to visualize the
//! [`Frustum`](bevy::camera::primitives::Frustum) of a camera.
//! Both can be used together to visualize which entities have been culled
//! from a given camera's view, which entities are visible, and when
//! that change happens during an entity's Aabb interaction with a camera's Frustum.
//!
//! This example contains a scene with a camera [`MyCamera`] that has its
//! [`Frustum`](bevy::camera::primitives::Frustum) gizmo visible.
//! A collection of [`MyShape`]s, with their individual
//! [`Aabb`](bevy::camera::primitives::Aabb) gizmos visible, periodically move in and
//! out of the camera's frustum. The [`Aabb`](bevy::camera::primitives::Aabb)
//! gizmos are colored red when they have been culled from [`MyCamera`]'s view.
//! The gizmos change color to green when the shape is considered visible by the
//! camera and would be extracted for rendering.
//!
//! A second active camera, controllable via the [`FreeCameraPlugin`], is used to observe the scene.
//! This second camera's view occupies most of the window. [`MyCamera`]'s view is visible in the
//! bottom right ninth of the screen.

use bevy::{
    camera::{
        visibility::{VisibilitySystems, VisibleEntities},
        Viewport,
    },
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState},
    gizmos::aabb::ShowAabbGizmo,
    input::common_conditions::input_just_pressed,
    prelude::*,
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_shapes,
                move_free_camera_to_my_camera.run_if(input_just_pressed(KeyCode::Digit1)),
                move_free_camera_to_original_position.run_if(input_just_pressed(KeyCode::Digit2)),
            ),
        )
        .add_systems(
            // Frustum culling happens in PostUpdate.
            // Our system will update the color of aabb's upon reading
            // the results of frustum culling after CheckVisibility runs.
            PostUpdate,
            update_shape_aabb_colors.after(VisibilitySystems::CheckVisibility),
        )
        .run();
}

/// A marker component for the ring some shapes will rotate on
#[derive(Component)]
struct ShapeRing;

/// A marker component for our shapes so they can be queried separately from the planes.
/// The `ShowAabbGizmo` component will be automatically added to `MyShape` to make their Aabbs
/// visible.
#[derive(Component, Default)]
#[require(ShowAabbGizmo)]
struct MyShape;

/// A marker component for the shape behind the wall.
#[derive(Component)]
#[require(MyShape)]
struct WallShape;

/// A marker component for the camera that is being debugged
/// The `ShowFrustumGizmo` component will be automatically added to `MyCamera` to make
/// its view frustum visible.
#[derive(Component)]
#[require(ShowFrustumGizmo)]
struct MyCamera;

const SHAPE_RING_RADIUS: f32 = 10.0;
const WALL_SHAPE_TIMER_DURATION_SECS: f32 = 8.0;
const FREE_CAMERA_START_TRANSFORM: Transform = Transform::from_xyz(-20., 10., 22.);
const FREE_CAMERA_START_TARGET: Vec3 = Vec3::new(7., 1.5, 0.);

fn setup(
    mut commands: Commands,
    windows: Query<&Window>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let window = windows.single()?;
    // The camera that the user controls to observe the scene.
    let free_camera = commands
        .spawn((
            Camera3d::default(),
            FREE_CAMERA_START_TRANSFORM.looking_at(FREE_CAMERA_START_TARGET, Vec3::Y),
            FreeCamera::default(),
        ))
        .id();

    // The camera that we want to debug frustum culling for. This will be rendered
    // as a picture-in-picture in the lower right ninth of the screen.
    let my_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0., 1.5, 0.).looking_at(Vec3::new(1.0, 1.5, 0.), Vec3::Y),
            Camera {
                order: 1,
                // The camera-to-debug's view will be in the lower right ninth of the screen.
                viewport: Some(Viewport {
                    physical_position: window.physical_size() * 2 / 3,
                    physical_size: window.physical_size() / 3,
                    ..default()
                }),
                // Do not write the free camera's view rendering back into the P-I-P
                msaa_writeback: MsaaWriteback::Off,
                ..default()
            },
            MyCamera,
        ))
        .id();

    // Instructions placed on top of the free_camera view
    commands.spawn((
        UiTargetCamera(free_camera),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![(
            Text::new(
                "This example utilizes free camera controls i.e. move with WASD and mouse grab to change orientation.\n\
                Press '1' to move the free camera to where MyCamera is, matching its view frustum.\n\
                Press '2' to move the free camera to its initial position in the example.",
            ),
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                left: px(12),
                ..default()
            },
        )]
    ));
    // Label for the picture-in-picture view of MyCamera
    commands.spawn((
        UiTargetCamera(my_camera),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![(
            Text::new("View of MyCamera"),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(12),
                right: px(100),
                ..default()
            },
        )],
    ));

    // Green Floor Plane
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(SHAPE_RING_RADIUS * 4., SHAPE_RING_RADIUS * 4.),
            ),
        ),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // Blue Wall Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5., 5.))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.3, 0.5))),
        Transform::from_xyz(20., 2.5, 10.).with_rotation(Quat::from_rotation_z(PI / 2.)),
    ));
    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));

    // Configure all AABB's to have a default color of red
    let (_, aabb_gizmo_config) = config_store.config_mut::<AabbGizmoConfigGroup>();
    aabb_gizmo_config.default_color = Some(Color::LinearRgba(LinearRgba::RED));

    // Configure the shapes on the ring that will have their AABB's drawn and updated
    let white_matl = materials.add(Color::WHITE);
    let shapes = [
        meshes.add(Cuboid {
            half_size: Vec3::new(2., 0.5, 1.),
        }),
        meshes.add(Tetrahedron {
            vertices: [
                Vec3::new(3., 4., 3.),
                Vec3::new(-0.5, 4., -0.5),
                Vec3::new(-0.5, -0.5, 3.),
                Vec3::new(3., -0.5, -0.5),
            ],
        }),
        meshes.add(Cylinder {
            radius: 0.1,
            half_height: 1.5,
        }),
        meshes.add(Cuboid {
            half_size: Vec3::new(1., 0.1, 2.),
        }),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
    ];
    let shapes_len = shapes.len() as f32;
    let mut shape_ring = commands.spawn((Transform::default(), Visibility::default(), ShapeRing));
    for (i, shape) in shapes.into_iter().enumerate() {
        // Space the shapes out evenly along the ring
        let shape_angle = i as f32 * 2. * PI / shapes_len;
        let (s, c) = ops::sin_cos(shape_angle);
        let (x, z) = (SHAPE_RING_RADIUS * c, SHAPE_RING_RADIUS * s);
        shape_ring.with_child((
            Mesh3d(shape),
            MeshMaterial3d(white_matl.clone()),
            Transform::from_xyz(x, 1.5, z).with_rotation(Quat::from_rotation_x(-PI / 4.)),
            MyShape,
        ));
    }

    // Configure the shape that peeks out of the wall plane
    let wall_shape = meshes.add(Torus::default());
    commands.spawn((
        Mesh3d(wall_shape),
        MeshMaterial3d(white_matl.clone()),
        Transform::from_xyz(25., 1.5, 12.5).with_rotation(Quat::from_rotation_x(-PI / 4.)),
        WallShape,
    ));

    Ok(())
}

// A system that:
// - rotates shapes in place
// - moves the ring shapes in a circle around MyCamera
// - moves the wall shape up and down
fn move_shapes(
    time: Res<Time>,
    mut timer: Local<Timer>,
    mut ring_query: Query<&mut Transform, (With<ShapeRing>, Without<MyShape>)>,
    mut shape_query: Query<(&mut Transform, Has<WallShape>), (With<MyShape>, Without<ShapeRing>)>,
) -> Result {
    // Initialize the wall shape's movement timer on the first run.
    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(WALL_SHAPE_TIMER_DURATION_SECS, TimerMode::Repeating);
    }
    timer.tick(time.delta());
    let dt = time.delta_secs();

    // Rotate the shapes themselves on their own axis
    for (mut transform, has_wall_shape) in &mut shape_query {
        transform.rotate_y(dt / 2.);
        if has_wall_shape {
            // the wall shape moves up for 4 seconds and then down for 4 seconds.
            // it oscillates between y = 1.5 and 15.0
            transform.translation.y = if timer.elapsed_secs() < WALL_SHAPE_TIMER_DURATION_SECS / 2.0
            {
                1.5 + 15.0 * timer.elapsed_secs() / (WALL_SHAPE_TIMER_DURATION_SECS / 2.0)
            } else {
                1.5 + 15.0 * (WALL_SHAPE_TIMER_DURATION_SECS - timer.elapsed_secs())
                    / (WALL_SHAPE_TIMER_DURATION_SECS / 2.0)
            }
        }
    }

    // Rotate the ring
    let transform = &mut ring_query.single_mut()?;
    transform.rotate_y(dt / 3.);

    Ok(())
}

// A system that changes the color of the [`AabbGizmo`](bevy::gizmos::Aabb)
// if they are considered visible by the camera.
fn update_shape_aabb_colors(
    view_query: Query<&VisibleEntities, With<MyCamera>>,
    mut gizmo_query: Query<&mut ShowAabbGizmo, With<MyShape>>,
) -> Result {
    // Reset the color to use the config's default color
    for mut shape_gizmo in &mut gizmo_query {
        shape_gizmo.color = None;
    }

    // Query for the shape entities visible for this camera
    // Update the gizmo on any such shape entity to be green
    let visible_entities = view_query.single()?;
    for entity in visible_entities.entities.values().flatten() {
        if let Ok(mut shape_gizmo) = gizmo_query.get_mut(*entity) {
            shape_gizmo.color = Some(Color::LinearRgba(LinearRgba::GREEN));
        }
    }
    Ok(())
}

// A system that moves the free camera to `MyCamera`, matching its view frustum.
// From here, the camera orientation can be moved to more easily see the transition of
// entities' visibilities with respect to `MyCamera` by looking at the frustum edges.
fn move_free_camera_to_my_camera(
    view_query: Query<&Transform, With<MyCamera>>,
    free_camera_query: Query<
        (&mut Transform, &mut FreeCameraState),
        (With<Camera3d>, Without<MyCamera>),
    >,
) -> Result {
    let my_camera_transform = view_query.single()?;
    move_free_camera(*my_camera_transform, free_camera_query)
}

// A system that moves the free camera back to its starting position in the example.
fn move_free_camera_to_original_position(
    free_camera_query: Query<
        (&mut Transform, &mut FreeCameraState),
        (With<Camera3d>, Without<MyCamera>),
    >,
) -> Result {
    move_free_camera(
        FREE_CAMERA_START_TRANSFORM.looking_at(FREE_CAMERA_START_TARGET, Vec3::Y),
        free_camera_query,
    )
}

fn move_free_camera(
    new_transform: Transform,
    mut free_camera_query: Query<
        (&mut Transform, &mut FreeCameraState),
        (With<Camera3d>, Without<MyCamera>),
    >,
) -> Result {
    let (mut transform, mut state) = free_camera_query.single_mut()?;
    *transform = new_transform;

    // Update the yaw and pitch so that free camera orientation is updated correctly upon mouse grab
    let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    state.yaw = yaw;
    state.pitch = pitch;

    Ok(())
}
