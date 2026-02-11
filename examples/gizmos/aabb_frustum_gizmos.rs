//! This example demonstrates how to use aabb and frustum gizmos for frustum culling debugging.
//!
//! Aabb gizmos are used to visualize the [`Aabb`](bevy::camera::primitives::Aabb) of an entity.
//! Frustum gizmos are used to visualize the [`Frustum`](bevy::camera::primitives::Frustum) of a camera.
//! Both can be used together to visualize frustum culling for a given camera.
//!
//! This example shows a scene with a camera that has its [`Frustum`](bevy::camera::primitives::Frustum)
//! gizmo visible. A rotating collection of shapes, with their individual
//! [`Aabb`](bevy::camera::primitives::Aabb) gizmos visible, circle in and out of the
//! camera's frustum. The [`Aabb`](bevy::camera::primitives::Aabb) gizmos are red
//! when they have been culled. They change to green when the shape
//! is considered visible by this camera and would be extracted for rendering.
//! A second active camera, controllable via the FreeCameraPlugin, is used to observe the scene.

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
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, move_shapes)
        .add_systems(
            PostUpdate,
            detect_visibility_of_shapes.after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
        )
        .run();
}

/// A marker component for the ring our shapes will rotate on
#[derive(Component)]
struct ShapeRing;

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct MyShape;

/// A marker component for the camera we are debugging.
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
        // To visualize its frustum from the free camera, the `ShowFrustumGizmo` component is added
        ShowFrustumGizmo::default(),
        Camera {
            // Place this camera's rendering on top of the free camera's rendering
            order: 1,
            // The camera-to-debug's view will be in the lower right ninth of the screen.
            viewport: Some(Viewport {
                physical_position: window.physical_size() * 2 / 3,
                physical_size: window.physical_size() / 3,
                ..default()
            }),
            // Do not write back the free camera's rendering back into the PIP
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

    // Configure AABB's to be drawn and have a default color of red
    let (_, aabb_gizmo_config) = config_store.config_mut::<AabbGizmoConfigGroup>();
    aabb_gizmo_config.draw_all = true;
    aabb_gizmo_config.default_color = Some(Color::LinearRgba(LinearRgba::RED));

    // Configure the ring shapes that will have their AABB's drawn
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
            // The `ShowAabbGizmo` component is added here so that we can override the color easier
            // when the shape is considered visible
            ShowAabbGizmo::default(),
        ));
    }
    Ok(())
}

// A system that rotates shapes in place and also moves them in a circle around the camera
fn move_shapes(
    time: Res<Time>,
    mut ring_query: Query<&mut Transform, (With<ShapeRing>, Without<MyShape>)>,
    mut shape_query: Query<&mut Transform, (With<MyShape>, Without<ShapeRing>)>,
) -> Result {
    let dt = time.delta_secs();
    for mut transform in &mut shape_query {
        transform.rotate_y(dt / 2.);
    }
    let transform = &mut ring_query.single_mut()?;
    transform.rotate_y(dt / 3.);
    Ok(())
}

// A system that changes the color of the [`AabbGizmo`](bevy::gizmos::Aabb)
// if they are considered visible by the camera.
fn detect_visibility_of_shapes(
    view_query: Query<&VisibleEntities, With<MyCamera>>,
    mut gizmo_query: Query<&mut ShowAabbGizmo, With<MyShape>>,
) -> Result {
    // reset the color to the default color
    for mut gizmo in &mut gizmo_query {
        gizmo.color = None;
    }

    let visible_entities = view_query.single()?;
    for entity in visible_entities.entities.values().flatten() {
        if let Ok(mut gizmo) = gizmo_query.get_mut(*entity) {
            gizmo.color = Some(Color::LinearRgba(LinearRgba::GREEN));
        }
    }
    Ok(())
}
