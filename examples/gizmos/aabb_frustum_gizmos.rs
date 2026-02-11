//! This example demonstrates how to use AABB and Frustum gizmos for frustum culling debugging.
//!
//! Aabb gizmos are used to visualize the [`Aabb`](bevy::camera::primitives::Aabb) of an entity.
//! Frustum gizmos are used to visualize the [`Frustum`](bevy::camera::primitives::Frustum) of a camera.
//! Both can be used together to visualize frustum culling for a given camera.
//!
//! This example shows a scene with a camera `MyCamera` that has its
//! [`Frustum`](bevy::camera::primitives::Frustum) gizmo visible.
//! A rotating ring of shapes, with their individual [`Aabb`](bevy::camera::primitives::Aabb)
//! gizmos visible, circle in and out of the camera's frustum.
//! The [`Aabb`](bevy::camera::primitives::Aabb) gizmos are red by default.
//! They gizmos change color to green when the shape is considered visible by the
//! camera and would be extracted for rendering.
//!
//! A second active camera, controllable via the FreeCameraPlugin, is used to observe the scene.
//! This second camera's view takes up most of the window. `MyCamera`'s view takes up the
//! bottom right ninth of the screen.

use bevy::{
    camera::{
        visibility::{VisibilitySystems, VisibleEntities},
        Viewport,
    },
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    gizmos::aabb::ShowAabbGizmo,
    prelude::*,
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, move_shapes)
        .add_systems(
            // Frustum culling happens in PostUpdate.
            // Our system will change the color of aabb's upon detecting
            // the results of frustum culling after the last VisibilitySystem runs
            PostUpdate,
            update_shape_aabb_colors.after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
        )
        .run();
}

/// A marker component for the ring our shapes will rotate on
#[derive(Component)]
struct ShapeRing;

/// A marker component for our shapes so they can be queried separately from the ground plane.
#[derive(Component)]
struct MyShape;

/// A marker component for the camera that is being debugged
#[derive(Component)]
struct MyCamera;

const SHAPE_RING_RADIUS: f32 = 10.0;

fn setup(
    mut commands: Commands,
    windows: Query<&Window>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let window = windows.single()?;
    // The camera that the user controls to observe the scene.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-20., 10., 22.).looking_at(Vec3::new(7., 1.5, 0.), Vec3::Y),
        FreeCamera::default(),
    ));

    // The camera that we want to debug frustum culling for. This will be rendered
    // as a picture-in-picture in the lower right ninth of the screen.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, 0.).looking_at(Vec3::new(1.0, 1.5, 0.), Vec3::Y),
        // To visualize its frustum from the free camera, the `ShowFrustumGizmo` component is added.
        ShowFrustumGizmo::default(),
        Camera {
            // Place this camera's rendering on top of the free camera's rendering.
            order: 1,
            // The camera-to-debug's view will be in the lower right ninth of the screen.
            viewport: Some(Viewport {
                physical_position: window.physical_size() * 2 / 3,
                physical_size: window.physical_size() / 3,
                ..default()
            }),
            // Do not write the free camera's view rendering back into the P-I-P
            msaa_writeback: MsaaWriteback::Off,
            ..Default::default()
        },
        MyCamera,
    ));

    // Plane
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(SHAPE_RING_RADIUS * 2., SHAPE_RING_RADIUS * 2.),
            ),
        ),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Configure all AABB's to be drawn and have a default color of red
    let (_, aabb_gizmo_config) = config_store.config_mut::<AabbGizmoConfigGroup>();
    aabb_gizmo_config.draw_all = true;
    aabb_gizmo_config.default_color = Some(Color::LinearRgba(LinearRgba::RED));

    // Configure the ring shapes that will have their AABB's drawn and updated
    let white_matl = materials.add(Color::WHITE);
    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
    ];
    let mut shape_ring = commands.spawn((Transform::default(), Visibility::default(), ShapeRing));
    for (i, shape) in shapes.into_iter().enumerate() {
        let shape_angle = i as f32 * 2. * PI / 5.;
        let (s, c) = ops::sin_cos(shape_angle);
        let (x, z) = (SHAPE_RING_RADIUS * c, SHAPE_RING_RADIUS * s);
        shape_ring.with_child((
            Mesh3d(shape),
            MeshMaterial3d(white_matl.clone()),
            Transform::from_xyz(x, 1.5, z).with_rotation(Quat::from_rotation_x(-PI / 4.)),
            MyShape,
            // The `ShowAabbGizmo` component is added here so that we can override its color easier
            // in `update_shape_aabb_colors`
            ShowAabbGizmo::default(),
        ));
    }
    Ok(())
}

// A system that rotates shapes in place and also moves them in a circle around the camera.
fn move_shapes(
    time: Res<Time>,
    mut ring_query: Query<&mut Transform, (With<ShapeRing>, Without<MyShape>)>,
    mut shape_query: Query<&mut Transform, (With<MyShape>, Without<ShapeRing>)>,
) -> Result {
    let dt = time.delta_secs();
    // Rotate the shapes themselves on their own axis
    for mut transform in &mut shape_query {
        transform.rotate_y(dt / 2.);
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
    // Reset the color to the default color
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
