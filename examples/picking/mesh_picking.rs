//! A simple 3D scene to demonstrate mesh picking.
//!
//! By default, all meshes are pickable. Picking can be disabled for individual entities
//! by adding [`PickingBehavior::IGNORE`].
//!
//! If you want mesh picking to be entirely opt-in, you can set [`MeshPickingBackendSettings::require_markers`]
//! to `true` and add a [`RayCastPickable`] component to the desired camera and target entities.

use std::f32::consts::PI;

use bevy::{
    color::palettes::{
        css::{PINK, RED, SILVER},
        tailwind::{CYAN_300, YELLOW_300},
    },
    picking::backend::PointerHits,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SceneMaterials>()
        .add_systems(Startup, setup)
        .add_systems(Update, (on_mesh_hover, rotate))
        .run();
}

/// Materials for the scene
#[derive(Resource, Default)]
struct SceneMaterials {
    pub white: Handle<StandardMaterial>,
    pub ground: Handle<StandardMaterial>,
    pub hover: Handle<StandardMaterial>,
    pub pressed: Handle<StandardMaterial>,
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
    mut scene_materials: ResMut<'_, SceneMaterials>,
) {
    // Set up the materials.
    scene_materials.white = materials.add(Color::WHITE);
    scene_materials.ground = materials.add(Color::from(SILVER));
    scene_materials.hover = materials.add(Color::from(CYAN_300));
    scene_materials.pressed = materials.add(Color::from(YELLOW_300));

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    let num_shapes = shapes.len();

    // Spawn the shapes. The meshes will be pickable by default.
    for (i, shape) in shapes.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(scene_materials.white.clone()),
                Transform::from_xyz(
                    -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                    2.0,
                    Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
            ))
            .observe(on_pointer_over)
            .observe(on_pointer_out)
            .observe(on_pointer_down)
            .observe(on_pointer_up);
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(scene_materials.white.clone()),
                Transform::from_xyz(
                    -EXTRUSION_X_EXTENT / 2.
                        + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                    2.0,
                    -Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
            ))
            .observe(on_pointer_over)
            .observe(on_pointer_out)
            .observe(on_pointer_down)
            .observe(on_pointer_up);
    }

    // Disable picking for the ground plane.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(scene_materials.ground.clone()),
        PickingBehavior::IGNORE,
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Instructions
    commands.spawn((
        Text::new("Hover over the shapes to pick them"),
        Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Changes the material when the pointer is over the mesh.
fn on_pointer_over(
    trigger: Trigger<'_, Pointer<Over>>,
    scene_materials: Res<'_, SceneMaterials>,
    mut query: Query<'_, '_, &mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Ok(mut material) = query.get_mut(trigger.entity()) {
        material.0 = scene_materials.hover.clone();
    }
}

/// Resets the material when the pointer leaves the mesh.
fn on_pointer_out(
    trigger: Trigger<'_, Pointer<Out>>,
    scene_materials: Res<'_, SceneMaterials>,
    mut query: Query<'_, '_, &mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Ok(mut material) = query.get_mut(trigger.entity()) {
        material.0 = scene_materials.white.clone();
    }
}

/// Changes the material when the pointer is pressed.
fn on_pointer_down(
    trigger: Trigger<'_, Pointer<Down>>,
    scene_materials: Res<'_, SceneMaterials>,
    mut query: Query<'_, '_, &mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Ok(mut material) = query.get_mut(trigger.entity()) {
        material.0 = scene_materials.pressed.clone();
    }
}

/// Resets the material when the pointer is released.
fn on_pointer_up(
    trigger: Trigger<'_, Pointer<Up>>,
    scene_materials: Res<'_, SceneMaterials>,
    mut query: Query<'_, '_, &mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Ok(mut material) = query.get_mut(trigger.entity()) {
        material.0 = scene_materials.hover.clone();
    }
}

/// Draws the closest point of intersection for pointer hits.
fn on_mesh_hover(
    mut pointer_hits: EventReader<'_, '_, PointerHits>,
    meshes: Query<'_, '_, Entity, With<Mesh3d>>,
    mut gizmos: Gizmos,
) {
    for hit in pointer_hits.read() {
        // Get the first mesh hit.
        // The hits are sorted by distance from the camera, so this is the closest hit.
        let Some(closest_hit) = hit
            .picks
            .iter()
            .filter_map(|(entity, hit)| meshes.get(*entity).map(|_| hit).ok())
            .next()
        else {
            continue;
        };

        let (Some(point), Some(normal)) = (closest_hit.position, closest_hit.normal) else {
            return;
        };

        gizmos.sphere(point, 0.05, RED);
        gizmos.arrow(point, point + normal * 0.5, PINK);
    }
}

/// Rotates the shapes.
fn rotate(mut query: Query<'_, '_, &mut Transform, With<Shape>>, time: Res<'_, Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}
