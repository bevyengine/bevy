//! This example demonstrates how you can add your own custom primitives to bevy highlighting
//! traits you may want to implement for your primitives to achieve different functionalities.

use std::f32::consts::{PI, SQRT_2};

use bevy::{
    color::palettes::css::{RED, WHITE},
    input::common_conditions::input_just_pressed,
    math::bounding::{
        Aabb2d, Bounded2d, Bounded3d, BoundedExtrusion, BoundingCircle, BoundingVolume,
    },
    prelude::*,
    render::{
        camera::ScalingMode,
        mesh::{Extrudable, ExtrusionBuilder, PerimeterSegment},
        render_asset::RenderAssetUsages,
    },
};

const HEART: Heart = Heart::new(0.5);
const EXTRUSION: Extrusion<Heart> = Extrusion {
    base_shape: Heart::new(0.5),
    half_depth: 0.5,
};

// The transform of the camera in 2D
const TRANSFORM_2D: Transform = Transform {
    translation: Vec3::ZERO,
    rotation: Quat::IDENTITY,
    scale: Vec3::ONE,
};
// The projection used for the camera in 2D
const PROJECTION_2D: Projection = Projection::Orthographic(OrthographicProjection {
    near: -1.0,
    far: 10.0,
    scale: 1.0,
    viewport_origin: Vec2::new(0.5, 0.5),
    scaling_mode: ScalingMode::AutoMax {
        max_width: 8.0,
        max_height: 20.0,
    },
    area: Rect {
        min: Vec2::NEG_ONE,
        max: Vec2::ONE,
    },
});

// The transform of the camera in 3D
const TRANSFORM_3D: Transform = Transform {
    translation: Vec3::ZERO,
    // The camera is pointing at the 3D shape
    rotation: Quat::from_xyzw(-0.14521316, -0.0, -0.0, 0.98940045),
    scale: Vec3::ONE,
};
// The projection used for the camera in 3D
const PROJECTION_3D: Projection = Projection::Perspective(PerspectiveProjection {
    fov: PI / 4.0,
    near: 0.1,
    far: 1000.0,
    aspect_ratio: 1.0,
});

/// State for tracking the currently displayed shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum CameraActive {
    #[default]
    /// The 2D shape is displayed
    Dim2,
    /// The 3D shape is displayed
    Dim3,
}

/// State for tracking the currently displayed shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum BoundingShape {
    #[default]
    /// No bounding shapes
    None,
    /// The bounding sphere or circle of the shape
    BoundingSphere,
    /// The Axis Aligned Bounding Box (AABB) of the shape
    BoundingBox,
}

/// A marker component for our 2D shapes so we can query them separately from the camera
#[derive(Component)]
struct Shape2d;

/// A marker component for our 3D shapes so we can query them separately from the camera
#[derive(Component)]
struct Shape3d;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<BoundingShape>()
        .init_state::<CameraActive>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (rotate_2d_shapes, bounding_shapes_2d).run_if(in_state(CameraActive::Dim2)),
                (rotate_3d_shapes, bounding_shapes_3d).run_if(in_state(CameraActive::Dim3)),
                update_bounding_shape.run_if(input_just_pressed(KeyCode::KeyB)),
                switch_cameras.run_if(input_just_pressed(KeyCode::Space)),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the camera
    commands.spawn(Camera3dBundle {
        transform: TRANSFORM_2D,
        projection: PROJECTION_2D,
        ..Default::default()
    });

    // Spawn the 2D heart
    commands.spawn((
        PbrBundle {
            // We can use the methods defined on the meshbuilder to customize the mesh.
            mesh: meshes.add(HEART.mesh().resolution(50)),
            material: materials.add(StandardMaterial {
                emissive: RED.into(),
                base_color: RED.into(),
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Shape2d,
    ));

    // Spawn an extrusion of the heart.
    commands.spawn((
        PbrBundle {
            transform: Transform::from_xyz(0., -3., -10.)
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            // We can set a custom resolution for the round parts of the extrusion aswell.
            mesh: meshes.add(EXTRUSION.mesh().resolution(50)),
            material: materials.add(StandardMaterial {
                base_color: RED.into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        Shape3d,
    ));

    // Point light for 3D
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 12.0, 1.0),
        ..default()
    });

    // Example instructions
    commands.spawn(
        TextBundle::from_section(
            "Press 'B' to toggle between no bounding shapes, bounding boxes (AABBs) and bounding spheres / circles\n\
            Press 'Space' to switch between 3D and 2D",
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

// Rotate the 2D shapes.
fn rotate_2d_shapes(mut shapes: Query<&mut Transform, With<Shape2d>>, time: Res<Time>) {
    let elapsed_seconds = time.elapsed_seconds();

    for mut transform in shapes.iter_mut() {
        transform.rotation = Quat::from_rotation_z(elapsed_seconds);
    }
}

// Draw bounding boxes or circles for the 2D shapes.
fn bounding_shapes_2d(
    shapes: Query<&Transform, With<Shape2d>>,
    mut gizmos: Gizmos,
    bounding_shape: Res<State<BoundingShape>>,
) {
    for transform in shapes.iter() {
        // Get the rotation angle from the 3D rotation.
        let rotation = transform.rotation.to_scaled_axis().z;

        match bounding_shape.get() {
            BoundingShape::None => (),
            BoundingShape::BoundingBox => {
                // Get the AABB of the primitive with the rotation and translation of the mesh.
                let aabb = HEART.aabb_2d(transform.translation.xy(), rotation);

                gizmos.rect_2d(aabb.center(), 0., aabb.half_size() * 2., WHITE);
            }
            BoundingShape::BoundingSphere => {
                // Get the bounding sphere of the primitive with the rotation and translation of the mesh.
                let bounding_circle = HEART.bounding_circle(transform.translation.xy(), rotation);

                gizmos
                    .circle_2d(bounding_circle.center(), bounding_circle.radius(), WHITE)
                    .resolution(64);
            }
        }
    }
}

// Rotate the 3D shapes.
fn rotate_3d_shapes(mut shapes: Query<&mut Transform, With<Shape3d>>, time: Res<Time>) {
    let delta_seconds = time.delta_seconds();

    for mut transform in shapes.iter_mut() {
        transform.rotate_y(delta_seconds);
    }
}

// Draw the AABBs or bounding spheres for the 3D shapes.
fn bounding_shapes_3d(
    shapes: Query<&Transform, With<Shape3d>>,
    mut gizmos: Gizmos,
    bounding_shape: Res<State<BoundingShape>>,
) {
    for transform in shapes.iter() {
        match bounding_shape.get() {
            BoundingShape::None => (),
            BoundingShape::BoundingBox => {
                // Get the AABB of the extrusion with the rotation and translation of the mesh.
                let aabb = EXTRUSION.aabb_3d(transform.translation, transform.rotation);

                gizmos.primitive_3d(
                    &Cuboid::from_size(Vec3::from(aabb.half_size()) * 2.),
                    aabb.center().into(),
                    Quat::IDENTITY,
                    WHITE,
                );
            }
            BoundingShape::BoundingSphere => {
                // Get the bounding sphere of the extrusion with the rotation and translation of the mesh.
                let bounding_sphere =
                    EXTRUSION.bounding_sphere(transform.translation, transform.rotation);

                gizmos.sphere(
                    bounding_sphere.center().into(),
                    Quat::IDENTITY,
                    bounding_sphere.radius(),
                    WHITE,
                );
            }
        }
    }
}

// Switch to the next bounding shape.
fn update_bounding_shape(
    current: Res<State<BoundingShape>>,
    mut next: ResMut<NextState<BoundingShape>>,
) {
    next.set(match current.get() {
        BoundingShape::None => BoundingShape::BoundingBox,
        BoundingShape::BoundingBox => BoundingShape::BoundingSphere,
        BoundingShape::BoundingSphere => BoundingShape::None,
    });
}

// Switch between 2D and 3D cameras.
fn switch_cameras(
    current: Res<State<CameraActive>>,
    mut next: ResMut<NextState<CameraActive>>,
    mut camera: Query<(&mut Transform, &mut Projection)>,
) {
    let next_state = match current.get() {
        CameraActive::Dim2 => CameraActive::Dim3,
        CameraActive::Dim3 => CameraActive::Dim2,
    };
    next.set(next_state);

    let (mut transform, mut projection) = camera.single_mut();
    match next_state {
        CameraActive::Dim2 => {
            *transform = TRANSFORM_2D;
            *projection = PROJECTION_2D;
        }
        CameraActive::Dim3 => {
            *transform = TRANSFORM_3D;
            *projection = PROJECTION_3D;
        }
    };
}

/// A custom 2D heart primitive. The heart is made up of two circles centered at `Vec2::new(±radius, 0.)` each with the same `radius`.
/// The tip of the heart connects the two circles at a 45° angle from `Vec3::NEG_Y`.
#[derive(Copy, Clone)]
struct Heart {
    /// The radius of each wing of the heart
    radius: f32,
}

// The `Primitive2d` or `Primitive3d` trait is required by almost all other traits for primitives in bevy.
// Depending on your shape, you should implement either one of them.
impl Primitive2d for Heart {}

impl Heart {
    const fn new(radius: f32) -> Self {
        Self { radius }
    }
}

// The `Measured2d` and `Measured3d` traits are used to compute the perimeter, the area or the volume of a primitive.
// If you implement `Measured2d` for a 2D primitive, `Measured3d` is automatically implemented for `Extrusion<T>`.
impl Measured2d for Heart {
    fn perimeter(&self) -> f32 {
        self.radius * (2.5 * PI + 2f32.powf(1.5) + 2.0)
    }

    fn area(&self) -> f32 {
        let circle_area = PI * self.radius * self.radius;
        let triangle_area = self.radius * self.radius * (1.0 + 2f32.sqrt()) / 2.0;
        let cutout = triangle_area - circle_area * 3.0 / 16.0;

        2.0 * circle_area + 4.0 * cutout
    }
}

// The `Bounded2d` or `Bounded3d` traits are used to compute the Axis Aligned Bounding Boxes or bounding circles / spheres for primitives.
impl Bounded2d for Heart {
    fn aabb_2d(&self, translation: Vec2, rotation: impl Into<Rot2>) -> Aabb2d {
        let rotation = rotation.into();
        // The center of the circle at the center of the right wing of the heart
        let circle_center = rotation * Vec2::new(self.radius, 0.0);
        // The maximum X and Y positions of the two circles of the wings of the heart.
        let max_circle = circle_center.abs() + Vec2::splat(self.radius);
        // Since the two circles of the heart are mirrored around the origin, the minimum position is the negative of the maximum.
        let min_circle = -max_circle;

        // The position of the tip at the bottom of the heart
        let tip_position = rotation * Vec2::new(0.0, -self.radius * (1. + SQRT_2));

        Aabb2d {
            min: translation + min_circle.min(tip_position),
            max: translation + max_circle.max(tip_position),
        }
    }

    fn bounding_circle(&self, translation: Vec2, rotation: impl Into<Rot2>) -> BoundingCircle {
        // The bounding circle of the heart is not at its origin. This `offset` is the offset between the center of the bounding circle and its translation.
        let offset = self.radius / 2f32.powf(1.5);
        // The center of the bounding circle
        let center = translation + rotation.into() * Vec2::new(0.0, -offset);
        // The radius of the bounding circle
        let radius = self.radius * (1.0 + 2f32.sqrt()) - offset;

        BoundingCircle::new(center, radius)
    }
}
// You can implement the `BoundedExtrusion` trait to implement `Bounded3d for Extrusion<Heart>`. There is a default implementation for both AABBs and bounding spheres,
// but you may be able to find faster solutions for your specific primitives.
impl BoundedExtrusion for Heart {}

// You can use the `Meshable` trait to create a `MeshBuilder` for the primitive.
impl Meshable for Heart {
    // The meshbuilder can be used to create the actual mesh for that primitive.
    type Output = HeartMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Self::Output {
            heart: *self,
            resolution: 32,
        }
    }
}

// You can include any additional information needed for meshing the primitive in the meshbuilder.
struct HeartMeshBuilder {
    heart: Heart,
    // The resolution determines the amount of vertices used for each wing of the heart
    resolution: usize,
}

// This trait is needed so that the configuration methods of the builder of the primitive are also available for the builder for the extrusion.
// If you do not want to support these configuration options for extrusions you can just implement them for your 2D mesh builder.
trait HeartBuilder {
    /// Set the resolution for each of the wings of the heart.
    fn resolution(self, resolution: usize) -> Self;
}

impl HeartBuilder for HeartMeshBuilder {
    fn resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution;
        self
    }
}

impl HeartBuilder for ExtrusionBuilder<Heart> {
    fn resolution(mut self, resolution: usize) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl MeshBuilder for HeartMeshBuilder {
    // This is where you should build the actual mesh.
    fn build(&self) -> Mesh {
        let radius = self.heart.radius;
        // The curved parts of each wing (half) of the heart have an angle of `PI * 1.25` or 225°
        let wing_angle = PI * 1.25;

        // We create buffers for the vertices, their normals and UVs, as well as the indices used to connect the vertices.
        let mut vertices = Vec::with_capacity(2 * self.resolution);
        let mut uvs = Vec::with_capacity(2 * self.resolution);
        let mut indices = Vec::with_capacity(6 * self.resolution - 9);
        // Since the heart is flat, we know all the normals are identical already.
        let normals = vec![[0f32, 0f32, 1f32]; 2 * self.resolution];

        // The point in the middle of the two curved parts of the heart
        vertices.push([0.0; 3]);
        uvs.push([0.5, 0.5]);

        // The left wing of the heart, starting from the point in the middle.
        for i in 1..self.resolution {
            let angle = (i as f32 / self.resolution as f32) * wing_angle;
            let (sin, cos) = angle.sin_cos();
            vertices.push([radius * (cos - 1.0), radius * sin, 0.0]);
            uvs.push([0.5 - (cos - 1.0) / 4., 0.5 - sin / 2.]);
        }

        // The bottom tip of the heart
        vertices.push([0.0, radius * (-1. - SQRT_2), 0.0]);
        uvs.push([0.5, 1.]);

        // The right wing of the heart, starting from the bottom most point and going towards the middle point.
        for i in 0..self.resolution - 1 {
            let angle = (i as f32 / self.resolution as f32) * wing_angle - PI / 4.;
            let (sin, cos) = angle.sin_cos();
            vertices.push([radius * (cos + 1.0), radius * sin, 0.0]);
            uvs.push([0.5 - (cos + 1.0) / 4., 0.5 - sin / 2.]);
        }

        // This is where we build all the triangles from the points created above.
        // Each triangle has one corner on the middle point with the other two being adjacent points on the perimeter of the heart.
        for i in 2..2 * self.resolution as u32 {
            indices.extend_from_slice(&[i - 1, i, 0]);
        }

        // Here, the actual `Mesh` is created. We set the indices, vertices, normals and UVs created above and specify the topology of the mesh.
        Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

// The `Extrudable` trait can be used to easily implement meshing for extrusions.
impl Extrudable for HeartMeshBuilder {
    fn perimeter(&self) -> Vec<bevy::render::mesh::PerimeterSegment> {
        let resolution = self.resolution as u32;
        vec![
            // The left wing of the heart
            PerimeterSegment::Smooth {
                // The normals of the first and last vertices of smooth segments have to be specified manually.
                first_normal: Vec2::X,
                last_normal: Vec2::new(-1.0, -1.0).normalize(),
                // These indices are used to index into the `ATTRIBUTE_POSITION` vec of your 2D mesh.
                indices: (0..resolution).collect(),
            },
            // The bottom tip of the heart
            PerimeterSegment::Flat {
                indices: vec![resolution - 1, resolution, resolution + 1],
            },
            // The right wing of the heart
            PerimeterSegment::Smooth {
                first_normal: Vec2::new(1.0, -1.0).normalize(),
                last_normal: Vec2::NEG_X,
                indices: (resolution + 1..2 * resolution).chain([0]).collect(),
            },
        ]
    }
}
