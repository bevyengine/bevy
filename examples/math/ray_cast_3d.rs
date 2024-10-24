//! Demonstrates ray casting for primitive shapes in 3D.
//!
//! Note that this is only intended to showcase the core ray casting methods for primitive shapes,
//! not how to perform large-scale ray casting in a real application.
//!
//! There are many optimizations that could be done, such as checking for intersections with bounding boxes before checking
//! for intersections with the actual shapes, and using an acceleration structure such as a Bounding Volume Hierarchy (BVH)
//! to speed up ray queries in large worlds.

use std::f32::consts::FRAC_PI_4;

use bevy::{color::palettes::css::*, prelude::*, window::PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_gizmo_config(
            DefaultGizmoConfigGroup,
            GizmoConfig {
                line_width: 3.0,
                ..default()
            },
        )
        .init_resource::<CursorRay>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_cursor_ray, rotate_shapes, ray_cast).chain())
        .run();
}

/// The world-space ray that is being cast from the cursor position.
#[derive(Resource, Deref, DerefMut)]
struct CursorRay(Ray3d);

impl Default for CursorRay {
    fn default() -> Self {
        Self(Ray3d::new(Vec3::ZERO, Vec3::NEG_Z))
    }
}

#[derive(Component)]
struct AngularVelocity(f32);

const SHAPE_COUNT: u32 = 9;
const SHAPES_X_EXTENT: f32 = 14.0;

/// An enum for supported 3D shapes.
///
/// Various trait implementations can be found at the bottom of this file.
#[derive(Component, Clone, Debug)]
#[allow(missing_docs)]
pub enum Shape3d {
    Sphere(Sphere),
    Cuboid(Cuboid),
    Cylinder(Cylinder),
    Cone(Cone),
    ConicalFrustum(ConicalFrustum),
    Capsule(Capsule3d),
    Triangle(Triangle3d),
    Tetrahedron(Tetrahedron),
    Torus(Torus),
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shapes = [
        Shape3d::Sphere(Sphere::default()),
        Shape3d::Cuboid(Cuboid::default()),
        Shape3d::Cylinder(Cylinder::default()),
        Shape3d::Cone(Cone::default()),
        Shape3d::ConicalFrustum(ConicalFrustum::default()),
        Shape3d::Capsule(Capsule3d::default()),
        Shape3d::Torus(Torus::default()),
        Shape3d::Triangle(Triangle3d::default()),
        Shape3d::Tetrahedron(Tetrahedron::default()),
    ];

    let material = materials.add(Color::WHITE);

    // Spawn the shapes
    for i in 0..SHAPE_COUNT {
        spawn_shape(
            &mut commands,
            &mut meshes,
            &material,
            shapes[i as usize].clone(),
            i as usize,
        );
    }

    // Spawn camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 6., 10.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Spawn light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 20_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Spawn instructions
    commands.spawn(
        TextBundle::from_section(
            "Point the cursor at the shapes to cast rays.",
            TextStyle::default(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

/// Spawns a shape at a given column and row.
fn spawn_shape(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    shape: Shape3d,
    column: usize,
) {
    commands.spawn((
        shape.clone(),
        Mesh3d(meshes.add(shape)),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(
            -SHAPES_X_EXTENT / 2. + column as f32 / (SHAPE_COUNT - 1) as f32 * SHAPES_X_EXTENT,
            2.0,
            0.0,
        )
        .with_rotation(Quat::from_rotation_x(-FRAC_PI_4)),
        AngularVelocity(0.5),
    ));
}

/// Rotates the shapes.
fn rotate_shapes(mut query: Query<(&mut Transform, &AngularVelocity)>, time: Res<Time>) {
    for (mut transform, ang_vel) in &mut query {
        transform.rotate_y(time.delta_seconds() * ang_vel.0);
    }
}

/// Moves `CursorRay` to follow the cursor.
fn update_cursor_ray(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut cursor_ray: ResMut<CursorRay>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera.single();

    if let Some(ray) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
    {
        cursor_ray.0 = ray;
    }
}

/// Performs ray casts against all shapes in the scene.
fn ray_cast(query: Query<(&Shape3d, &Transform)>, mut gizmos: Gizmos, ray: Res<CursorRay>) {
    let max_distance = 10_000.0;

    let mut closest_hit = None;
    let mut closest_hit_distance = f32::MAX;

    // Iterate over all shapes.
    // NOTE: A more efficient implementation would use an acceleration structure such as
    //       a Bounding Volume Hierarchy (BVH), and test the ray against bounding boxes first.
    for (shape, transform) in &query {
        // Cast the ray against the shape transformed by the isometry.
        // NOTE: This method is provided by the `PrimitiveRayCast3d` trait.
        let Some(hit) = shape.ray_cast(transform.to_isometry(), ray.0, max_distance, false) else {
            continue;
        };

        if hit.distance < closest_hit_distance {
            closest_hit = Some((ray.get_point(hit.distance), hit.normal));
            closest_hit_distance = hit.distance;
        }
    }

    // Draw the closest hit point.
    if let Some((point, normal)) = closest_hit {
        // Normal
        gizmos
            .arrow(point, point + *normal, RED)
            .with_tip_length(0.1);

        // Hit point
        let iso = Isometry3d::from_translation(point);
        gizmos.sphere(iso, 0.030, ORANGE);
        gizmos.sphere(iso, 0.025, ORANGE);
        gizmos.sphere(iso, 0.020, ORANGE);
        gizmos.sphere(iso, 0.010, ORANGE);
        gizmos.sphere(iso, 0.010, ORANGE);
        gizmos.sphere(iso, 0.005, ORANGE);
    }
}

// Trait implementations for `Shape3d` to make ray casts and rendering shapes easier.

impl Primitive3d for Shape3d {}

impl PrimitiveRayCast3d for Shape3d {
    fn local_ray_cast(&self, ray: Ray3d, max_distance: f32, solid: bool) -> Option<RayHit3d> {
        use Shape3d::*;

        match self {
            Sphere(sphere) => sphere.local_ray_cast(ray, max_distance, solid),
            Cuboid(cuboid) => cuboid.local_ray_cast(ray, max_distance, solid),
            Cylinder(cylinder) => cylinder.local_ray_cast(ray, max_distance, solid),
            Cone(cone) => cone.local_ray_cast(ray, max_distance, solid),
            ConicalFrustum(frustum) => frustum.local_ray_cast(ray, max_distance, solid),
            Capsule(capsule) => capsule.local_ray_cast(ray, max_distance, solid),
            Triangle(triangle) => triangle.local_ray_cast(ray, max_distance, solid),
            Tetrahedron(tetrahedron) => tetrahedron.local_ray_cast(ray, max_distance, solid),
            Torus(torus) => torus.local_ray_cast(ray, max_distance, solid),
        }
    }
}

impl From<Shape3d> for Mesh {
    fn from(value: Shape3d) -> Self {
        use Shape3d::*;

        match value {
            Sphere(sphere) => sphere.into(),
            Cuboid(cuboid) => cuboid.into(),
            Cylinder(cylinder) => cylinder.into(),
            Cone(cone) => cone.into(),
            ConicalFrustum(frustum) => frustum.into(),
            Capsule(capsule) => capsule.into(),
            Triangle(triangle) => triangle.into(),
            Tetrahedron(tetrahedron) => tetrahedron.into(),
            Torus(torus) => torus.into(),
        }
    }
}
