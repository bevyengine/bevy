//! Shows how to render simple primitive shapes with a single color.

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

struct Shape {
    mesh: Mesh2dHandle,
    transform: Transform,
}

impl Shape {
    fn new(mesh: Handle<Mesh>, transform: Transform) -> Self {
        Self {
            mesh: Mesh2dHandle(mesh),
            transform,
        }
    }
}
impl From<Handle<Mesh>> for Shape {
    fn from(mesh: Handle<Mesh>) -> Self {
        Self::new(mesh, default())
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let shapes = [
        Shape::from(meshes.add(Circle { radius: 50.0 })),
        Shape::new(
            meshes.add(CircularSector::from_radians(50.0, 5.0)),
            // A sector is drawn symmetrically from the top.
            // To make it face right, we must rotate it to the left by 90 degrees.
            Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
        ),
        {
            let segment = CircularSegment::from_degrees(50.0, 135.0);
            Shape::new(
                meshes.add(segment),
                // The segment is drawn with the center as the center of the circle.
                // By subtracting the apothem, we move the segment down so that it touches the line x = 0.
                Transform::from_translation(Vec3::new(0.0, -segment.arc.apothem(), 0.0)),
            )
        },
        Shape::from(meshes.add(Ellipse::new(25.0, 50.0))),
        Shape::from(meshes.add(Capsule2d::new(25.0, 50.0))),
        Shape::from(meshes.add(Rectangle::new(50.0, 100.0))),
        Shape::from(meshes.add(RegularPolygon::new(50.0, 6))),
        Shape::from(meshes.add(Triangle2d::new(
            Vec2::Y * 50.0,
            Vec2::new(-50.0, -50.0),
            Vec2::new(50.0, -50.0),
        ))),
    ];
    let num_shapes = shapes.len();
    let x_extent = num_shapes as f32 * 100.0;

    for (i, shape) in shapes.into_iter().enumerate() {
        // Distribute colors evenly across the rainbow.
        let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

        let mut transform = shape.transform;
        // Distribute shapes from -x_extent to +x_extent.
        transform.translation.x += -x_extent / 2. + i as f32 / (num_shapes - 1) as f32 * x_extent;
        commands.spawn(MaterialMesh2dBundle {
            mesh: shape.mesh,
            material: materials.add(color),
            transform,
            ..default()
        });
    }
}
