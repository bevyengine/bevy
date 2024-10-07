//! Demonstrates ray casting for primitive shapes in 2D.
//!
//! Note that this is only intended to showcase the core ray casting methods for primitive shapes,
//! not how to perform large-scale ray casting in a real application.
//!
//! There are many optimizations that could be done, such as checking for intersections with bounding boxes before checking
//! for intersections with the actual shapes, and using an acceleration structure such as a Bounding Volume Hierarchy (BVH)
//! to speed up ray queries in large worlds.

use bevy::{
    color::palettes::{
        css::*,
        tailwind::{CYAN_600, LIME_500},
    },
    prelude::*,
    window::PrimaryWindow,
};

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
        .add_systems(
            Update,
            (draw_shapes, ray_follow_cursor, rotate_ray, ray_cast).chain(),
        )
        .run();
}

/// The world-space ray that is being cast from the cursor position.
#[derive(Resource, Deref, DerefMut)]
struct CursorRay(Ray2d);

impl Default for CursorRay {
    fn default() -> Self {
        Self(Ray2d::new(Vec2::ZERO, Vec2::Y))
    }
}

const X_EXTENT: f32 = 800.;
const Y_EXTENT: f32 = 150.;
const ROWS: u32 = 2;
const COLUMNS: u32 = 6;

/// An enum for supported 2D shapes.
///
/// Various trait implementations can be found at the bottom of this file.
#[derive(Component, Clone, Debug)]
#[allow(missing_docs)]
pub enum Shape2d {
    Circle(Circle),
    Arc(Arc2d),
    CircularSector(CircularSector),
    CircularSegment(CircularSegment),
    Ellipse(Ellipse),
    Annulus(Annulus),
    Rectangle(Rectangle),
    Rhombus(Rhombus),
    Line(Line2d),
    Segment(Segment2d),
    Polyline(BoxedPolyline2d),
    Polygon(BoxedPolygon),
    RegularPolygon(RegularPolygon),
    Triangle(Triangle2d),
    Capsule(Capsule2d),
}

fn setup(mut commands: Commands) {
    let shapes = [
        Shape2d::Circle(Circle::new(50.0)),
        Shape2d::Arc(Arc2d::new(50.0, 1.25)),
        Shape2d::CircularSector(CircularSector::new(50.0, 1.25)),
        Shape2d::CircularSegment(CircularSegment::new(50.0, 1.25)),
        Shape2d::Ellipse(Ellipse::new(25.0, 50.0)),
        Shape2d::Annulus(Annulus::new(25.0, 50.0)),
        Shape2d::Capsule(Capsule2d::new(25.0, 50.0)),
        Shape2d::Rectangle(Rectangle::new(50.0, 100.0)),
        Shape2d::Rhombus(Rhombus::new(75.0, 100.0)),
        Shape2d::RegularPolygon(RegularPolygon::new(50.0, 6)),
        Shape2d::Triangle(Triangle2d::new(
            Vec2::Y * 50.0,
            Vec2::new(-50.0, -50.0),
            Vec2::new(50.0, -50.0),
        )),
        Shape2d::Polygon(BoxedPolygon::new([
            Vec2::ZERO,
            Vec2::new(70.0, 45.0),
            Vec2::new(80.0, -50.0),
            Vec2::new(-60.0, -30.0),
            Vec2::new(-40.0, 60.0),
        ])),
        Shape2d::Segment(Segment2d::new(Dir2::from_xy(1.0, 0.5).unwrap(), 200.0)),
        Shape2d::Polyline(BoxedPolyline2d::new([
            Vec2::new(-120.0, -50.0),
            Vec2::new(-30.0, 30.0),
            Vec2::new(50.0, -40.0),
            Vec2::new(120.0, 50.0),
        ])),
        Shape2d::Line(Line2d {
            direction: Dir2::from_xy(1.0, -0.5).unwrap(),
        }),
    ];

    // Spawn two rows of shapes
    for i in 0..COLUMNS {
        for j in 0..ROWS {
            spawn_shape(
                &mut commands,
                shapes[(i + j * COLUMNS) as usize].clone(),
                i as usize,
                j as usize,
            );
        }
    }

    // Spawn remaining shapes at specific positions
    commands.spawn((shapes[12].clone(), Transform::from_xyz(-200.0, -250.0, 0.0)));
    commands.spawn((shapes[13].clone(), Transform::from_xyz(200.0, -250.0, 0.0)));
    commands.spawn((shapes[14].clone(), Transform::from_xyz(300.0, 250.0, 0.0)));

    // Spawn camera
    commands.spawn(Camera2d);

    // Spawn instructions
    commands.spawn(
        TextBundle::from_section(
            "Move the cursor to move the ray.\nLeft mouse button to rotate the ray counterclockwise.\nRight mouse button to rotate the ray clockwise.",
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
fn spawn_shape(commands: &mut Commands, shape: Shape2d, column: usize, row: usize) {
    commands.spawn((
        shape,
        Transform::from_xyz(
            -X_EXTENT / 2. + column as f32 / (COLUMNS - 1) as f32 * X_EXTENT,
            Y_EXTENT / 2. - row as f32 / (ROWS - 1) as f32 * Y_EXTENT,
            0.0,
        ),
    ));
}

/// Moves `CursorRay` to follow the cursor position.
fn ray_follow_cursor(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut ray: ResMut<CursorRay>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera.single();

    if let Some(cursor_world_pos) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        ray.origin = cursor_world_pos;
    }
}

/// Rotates the ray when the left or right mouse button is pressed.
fn rotate_ray(button: ResMut<ButtonInput<MouseButton>>, mut ray: ResMut<CursorRay>) {
    if button.pressed(MouseButton::Left) {
        ray.direction = Rot2::radians(0.015) * ray.direction;
    }
    if button.pressed(MouseButton::Right) {
        ray.direction = Rot2::radians(-0.015) * ray.direction;
    }
}

/// Performs ray casts against all shapes in the scene.
fn ray_cast(query: Query<(&Shape2d, &Transform)>, mut gizmos: Gizmos, ray: Res<CursorRay>) {
    let max_distance = 10_000.0;

    let mut closest_hit = None;
    let mut closest_hit_distance = f32::MAX;

    // Iterate over all shapes.
    // NOTE: A more efficient implementation would use an acceleration structure such as
    //       a Bounding Volume Hierarchy (BVH), and test the ray against bounding boxes first.
    for (shape, transform) in &query {
        let rotation = Rot2::radians(transform.rotation.to_euler(EulerRot::XYZ).2);
        let iso = Isometry2d::new(transform.translation.truncate(), rotation);

        // Cast the ray against the shape transformed by the isometry.
        // The shape is treated as hollow, meaning that the ray can intersect the shape's boundary from the inside.
        // NOTE: This method is provided by the `PrimitiveRayCast2d` trait.
        let Some(hit) = shape.ray_cast(iso, ray.0, max_distance, false) else {
            continue;
        };

        if hit.distance < closest_hit_distance {
            closest_hit = Some((ray.get_point(hit.distance), hit.normal));
            closest_hit_distance = hit.distance;
        }
    }

    // Draw the ray and the closest hit point.
    if let Some((point, normal)) = closest_hit {
        // Ray
        gizmos.line_2d(ray.origin, point, LIME_500);

        // Normal
        gizmos
            .arrow_2d(point, point + 50.0 * *normal, RED)
            .with_tip_length(5.0);

        // Hit point
        let iso = Isometry2d::from_translation(point);
        gizmos.circle_2d(iso, 3.0, ORANGE);
        gizmos.circle_2d(iso, 2.5, ORANGE);
        gizmos.circle_2d(iso, 2.0, ORANGE);
        gizmos.circle_2d(iso, 1.0, ORANGE);
        gizmos.circle_2d(iso, 1.0, ORANGE);
        gizmos.circle_2d(iso, 0.5, ORANGE);
    } else {
        gizmos.line_2d(ray.origin, ray.get_point(max_distance), CYAN_600);
    }
}

/// Draws all shapes in the scene.
fn draw_shapes(query: Query<(&Shape2d, &GlobalTransform)>, mut gizmos: Gizmos) {
    for (shape, global_transform) in &query {
        let transform = global_transform.compute_transform();
        let pos = transform.translation.truncate();
        let rot = Rot2::radians(transform.rotation.to_euler(EulerRot::XYZ).2);
        gizmos.primitive_2d(shape, Isometry2d::new(pos, rot), Color::WHITE);
    }
}

// Trait implementations for `Shape2d` to make ray casts and drawing shapes easier.

impl Primitive2d for Shape2d {}

impl PrimitiveRayCast2d for Shape2d {
    fn local_ray_cast(&self, ray: Ray2d, max_distance: f32, solid: bool) -> Option<RayHit2d> {
        use Shape2d::*;

        match self {
            Circle(circle) => circle.local_ray_cast(ray, max_distance, solid),
            Arc(arc) => arc.local_ray_cast(ray, max_distance, solid),
            CircularSector(sector) => sector.local_ray_cast(ray, max_distance, solid),
            CircularSegment(segment) => segment.local_ray_cast(ray, max_distance, solid),
            Ellipse(ellipse) => ellipse.local_ray_cast(ray, max_distance, solid),
            Annulus(annulus) => annulus.local_ray_cast(ray, max_distance, solid),
            Rectangle(rectangle) => rectangle.local_ray_cast(ray, max_distance, solid),
            Rhombus(rhombus) => rhombus.local_ray_cast(ray, max_distance, solid),
            Line(line) => line.local_ray_cast(ray, max_distance, solid),
            Segment(segment) => segment.local_ray_cast(ray, max_distance, solid),
            Polyline(polyline) => polyline.local_ray_cast(ray, max_distance, solid),
            Polygon(polygon) => polygon.local_ray_cast(ray, max_distance, solid),
            RegularPolygon(polygon) => polygon.local_ray_cast(ray, max_distance, solid),
            Triangle(triangle) => triangle.local_ray_cast(ray, max_distance, solid),
            Capsule(capsule) => capsule.local_ray_cast(ray, max_distance, solid),
        }
    }
}

impl<'w, 's, Config> GizmoPrimitive2d<Shape2d> for Gizmos<'w, 's, Config>
where
    Config: GizmoConfigGroup,
{
    type Output<'a> = () where Self: 'a;

    fn primitive_2d(
        &mut self,
        primitive: &Shape2d,
        isometry: Isometry2d,
        color: impl Into<Color>,
    ) -> Self::Output<'_> {
        use Shape2d::*;

        match &primitive {
            Circle(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Arc(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            CircularSector(shape) => self.primitive_2d(shape, isometry, color),
            CircularSegment(shape) => self.primitive_2d(shape, isometry, color),
            Ellipse(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Annulus(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Rectangle(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Rhombus(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Line(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Segment(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Polyline(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Polygon(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            RegularPolygon(shape) => self.primitive_2d(shape, isometry, color),
            Triangle(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
            Capsule(shape) => {
                self.primitive_2d(shape, isometry, color);
            }
        }
    }
}
