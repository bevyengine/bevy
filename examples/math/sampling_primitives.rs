//! This example shows how to sample random points from primitive shapes.

use std::f32::consts::PI;

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    math::prelude::*,
    prelude::*,
};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SampledShapes::new())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_mouse,
                handle_keypress,
                spawn_points,
                despawn_points,
                animate_spawning,
                animate_despawning,
                update_camera,
                update_lights,
            ),
        )
        .run();
}

// Constants

/// Maximum distance of the camera from its target. (meters)
/// Should be set such that it is possible to look at all objects
const MAX_CAMERA_DISTANCE: f32 = 12.0;

/// Minimum distance of the camera from its target. (meters)
/// Should be set such that it is not possible to clip into objects
const MIN_CAMERA_DISTANCE: f32 = 1.0;

/// Offset to be placed between the shapes
const DISTANCE_BETWEEN_SHAPES: Vec3 = Vec3::new(2.0, 0.0, 0.0);

/// Maximum amount of points allowed to be present.
/// Should be set such that it does not cause large amounts of lag when reached.
const MAX_POINTS: usize = 3000; // TODO: Test wasm and add a wasm-specific-bound

/// How many points should be spawned each frame
const POINTS_PER_FRAME: usize = 3;

/// Color used for the inside points
const INSIDE_POINT_COLOR: LinearRgba = LinearRgba::rgb(0.855, 1.1, 0.01);
/// Color used for the points on the boundary
const BOUNDARY_POINT_COLOR: LinearRgba = LinearRgba::rgb(0.08, 0.2, 0.90);

/// Time (in seconds) for the spawning/despawning animation
const ANIMATION_TIME: f32 = 1.0;

/// Color for the sky and the sky-light
const SKY_COLOR: Color = Color::srgb(0.02, 0.06, 0.15);

const SMALL_3D: f32 = 0.5;
const BIG_3D: f32 = 1.0;

// primitives

const CUBOID: Cuboid = Cuboid {
    half_size: Vec3::new(SMALL_3D, BIG_3D, SMALL_3D),
};

const SPHERE: Sphere = Sphere {
    radius: 1.5 * SMALL_3D,
};

const TRIANGLE_3D: Triangle3d = Triangle3d {
    vertices: [
        Vec3::new(BIG_3D, -BIG_3D * 0.5, 0.0),
        Vec3::new(0.0, BIG_3D, 0.0),
        Vec3::new(-BIG_3D, -BIG_3D * 0.5, 0.0),
    ],
};

const CAPSULE_3D: Capsule3d = Capsule3d {
    radius: SMALL_3D,
    half_length: SMALL_3D,
};

const CYLINDER: Cylinder = Cylinder {
    radius: SMALL_3D,
    half_height: SMALL_3D,
};

const TETRAHEDRON: Tetrahedron = Tetrahedron {
    vertices: [
        Vec3::new(-BIG_3D, -BIG_3D * 0.67, BIG_3D * 0.5),
        Vec3::new(BIG_3D, -BIG_3D * 0.67, BIG_3D * 0.5),
        Vec3::new(0.0, -BIG_3D * 0.67, -BIG_3D * 1.17),
        Vec3::new(0.0, BIG_3D, 0.0),
    ],
};

// Components, Resources

/// Resource for the random sampling mode, telling whether to sample the interior or the boundary.
#[derive(Resource)]
enum SamplingMode {
    Interior,
    Boundary,
}

/// Resource for storing whether points should spawn by themselves
#[derive(Resource)]
enum SpawningMode {
    Manual,
    Automatic,
}

/// Resource for tracking how many points should be spawned
#[derive(Resource)]
struct SpawnQueue(usize);

#[derive(Resource)]
struct PointCounter(usize);

/// Resource storing the shapes being sampled and their translations.
#[derive(Resource)]
struct SampledShapes(Vec<(Shape, Vec3)>);

impl SampledShapes {
    fn new() -> Self {
        let shapes = Shape::list_all_shapes();

        let n_shapes = shapes.len();

        let translations =
            (0..n_shapes).map(|i| (i as f32 - n_shapes as f32 / 2.0) * DISTANCE_BETWEEN_SHAPES);

        SampledShapes(shapes.into_iter().zip(translations).collect())
    }
}

/// Enum listing the shapes that can be sampled
#[derive(Clone, Copy)]
enum Shape {
    Cuboid,
    Sphere,
    Capsule,
    Cylinder,
    Tetrahedron,
    Triangle,
}
struct ShapeMeshBuilder {
    shape: Shape,
}

impl Shape {
    /// Return a vector containing all implemented shapes
    fn list_all_shapes() -> Vec<Shape> {
        vec![
            Shape::Cuboid,
            Shape::Sphere,
            Shape::Capsule,
            Shape::Cylinder,
            Shape::Tetrahedron,
            Shape::Triangle,
        ]
    }
}

impl ShapeSample for Shape {
    type Output = Vec3;
    fn sample_interior<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        match self {
            Shape::Cuboid => CUBOID.sample_interior(rng),
            Shape::Sphere => SPHERE.sample_interior(rng),
            Shape::Capsule => CAPSULE_3D.sample_interior(rng),
            Shape::Cylinder => CYLINDER.sample_interior(rng),
            Shape::Tetrahedron => TETRAHEDRON.sample_interior(rng),
            Shape::Triangle => TRIANGLE_3D.sample_interior(rng),
        }
    }

    fn sample_boundary<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> Self::Output {
        match self {
            Shape::Cuboid => CUBOID.sample_boundary(rng),
            Shape::Sphere => SPHERE.sample_boundary(rng),
            Shape::Capsule => CAPSULE_3D.sample_boundary(rng),
            Shape::Cylinder => CYLINDER.sample_boundary(rng),
            Shape::Tetrahedron => TETRAHEDRON.sample_boundary(rng),
            Shape::Triangle => TRIANGLE_3D.sample_boundary(rng),
        }
    }
}

impl Meshable for Shape {
    type Output = ShapeMeshBuilder;

    fn mesh(&self) -> Self::Output {
        ShapeMeshBuilder { shape: *self }
    }
}

impl MeshBuilder for ShapeMeshBuilder {
    fn build(&self) -> Mesh {
        match self.shape {
            Shape::Cuboid => CUBOID.mesh().into(),
            Shape::Sphere => SPHERE.mesh().into(),
            Shape::Capsule => CAPSULE_3D.mesh().into(),
            Shape::Cylinder => CYLINDER.mesh().into(),
            Shape::Tetrahedron => TETRAHEDRON.mesh().into(),
            Shape::Triangle => TRIANGLE_3D.mesh().into(),
        }
    }
}

/// The source of randomness used by this example.
#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

/// A container for the handle storing the mesh used to display sampled points as spheres.
#[derive(Resource)]
struct PointMesh(Handle<Mesh>);

/// A container for the handle storing the material used to display sampled points.
#[derive(Resource)]
struct PointMaterial {
    interior: Handle<StandardMaterial>,
    boundary: Handle<StandardMaterial>,
}

/// Marker component for sampled points.
#[derive(Component)]
struct SamplePoint;

/// Component for animating the spawn animation of lights.
#[derive(Component)]
struct SpawningPoint {
    progress: f32,
}

/// Marker component for lights which should change intensity.
#[derive(Component)]
struct DespawningPoint {
    progress: f32,
}

/// Marker component for lights which should change intensity.
#[derive(Component)]
struct FireflyLights;

/// The pressed state of the mouse, used for camera motion.
#[derive(Resource)]
struct MousePressed(bool);

/// Camera movement component.
#[derive(Component)]
struct CameraRig {
    /// Rotation around the vertical axis of the camera (radians).
    /// Positive changes makes the camera look more from the right.
    pub yaw: f32,
    /// Rotation around the horizontal axis of the camera (radians) (-pi/2; pi/2).
    /// Positive looks down from above.
    pub pitch: f32,
    /// Distance from the center, smaller distance causes more zoom.
    pub distance: f32,
    /// Location in 3D space at which the camera is looking and around which it is orbiting.
    pub target: Vec3,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shapes: Res<SampledShapes>,
) {
    // Use seeded rng and store it in a resource; this makes the random output reproducible.
    let seeded_rng = ChaCha8Rng::seed_from_u64(4); // Chosen by a fair die roll, guaranteed to be random.
    commands.insert_resource(RandomSource(seeded_rng));

    // Make a plane for establishing space.
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(20.0, 20.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -2.5, 0.0),
        ..default()
    });

    let shape_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.1, 0.6, 0.3),
        reflectance: 0.0,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    // Spawn shapes to be sampled
    for (shape, translation) in shapes.0.iter() {
        // The sampled shape shown transparently:
        commands.spawn(PbrBundle {
            mesh: meshes.add(shape.mesh()),
            material: shape_material.clone(),
            transform: Transform::from_translation(*translation),
            ..default()
        });

        // Lights which work as the bulk lighting of the fireflies:
        commands.spawn((
            PointLightBundle {
                point_light: PointLight {
                    range: 4.0,
                    radius: 0.6,
                    intensity: 1.0,
                    shadows_enabled: false,
                    color: Color::LinearRgba(INSIDE_POINT_COLOR),
                    ..default()
                },
                transform: Transform::from_translation(*translation),
                ..default()
            },
            FireflyLights,
        ));
    }

    // Global light:
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            color: SKY_COLOR,
            intensity: 2_000.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // A camera:
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true, // HDR is required for bloom
                clear_color: ClearColorConfig::Custom(SKY_COLOR),
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            transform: Transform::from_xyz(-2.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::NATURAL,
        CameraRig {
            yaw: 0.56,
            pitch: 0.45,
            distance: 8.0,
            target: Vec3::ZERO,
        },
    ));

    // Store the mesh and material for sample points in resources:
    commands.insert_resource(PointMesh(
        meshes.add(Sphere::new(0.03).mesh().ico(1).unwrap()),
    ));
    commands.insert_resource(PointMaterial {
        interior: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            reflectance: 0.05,
            emissive: 2.5 * INSIDE_POINT_COLOR,
            ..default()
        }),
        boundary: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            reflectance: 0.05,
            emissive: 1.5 * BOUNDARY_POINT_COLOR,
            ..default()
        }),
    });

    // Instructions for the example:
    commands.spawn(
        TextBundle::from_section(
            "Controls:\n\
            M: Toggle between sampling boundary and interior.\n\
            A: Toggle automatic spawning & despawning of points.\n\
            R: Restart (erase all samples).\n\
            S: Add one random sample.\n\
            D: Add 100 random samples.\n\
            Rotate camera by panning via mouse.\n\
            Zoom camera by scrolling via mouse or +/-.\n\
            Move camera by L/R arrow keys.\n\
            Tab: Toggle this text",
            TextStyle::default(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    // No points are scheduled to spawn initially.
    commands.insert_resource(SpawnQueue(0));

    // No points have been spawned initially.
    commands.insert_resource(PointCounter(0));

    // The mode starts with interior points.
    commands.insert_resource(SamplingMode::Interior);

    // Points spawn automatically by default.
    commands.insert_resource(SpawningMode::Automatic);

    // Starting mouse-pressed state is false.
    commands.insert_resource(MousePressed(false));
}

// Handle user inputs from the keyboard:
#[allow(clippy::too_many_arguments)]
fn handle_keypress(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<SamplingMode>,
    mut spawn_mode: ResMut<SpawningMode>,
    samples: Query<Entity, With<SamplePoint>>,
    shapes: Res<SampledShapes>,
    mut spawn_queue: ResMut<SpawnQueue>,
    mut counter: ResMut<PointCounter>,
    mut text_menus: Query<&mut Visibility, With<Text>>,
    mut camera: Query<&mut CameraRig>,
) {
    // R => restart, deleting all samples
    if keyboard.just_pressed(KeyCode::KeyR) {
        // Don't forget to zero out the counter!
        counter.0 = 0;
        for entity in &samples {
            commands.entity(entity).despawn();
        }
    }

    // S => sample once
    if keyboard.just_pressed(KeyCode::KeyS) {
        spawn_queue.0 += 1;
    }

    // D => sample a hundred
    if keyboard.just_pressed(KeyCode::KeyD) {
        spawn_queue.0 += 100;
    }

    // M => toggle mode between interior and boundary.
    if keyboard.just_pressed(KeyCode::KeyM) {
        match *mode {
            SamplingMode::Interior => *mode = SamplingMode::Boundary,
            SamplingMode::Boundary => *mode = SamplingMode::Interior,
        }
    }

    // A => toggle spawning mode between automatic and manual.
    if keyboard.just_pressed(KeyCode::KeyA) {
        match *spawn_mode {
            SpawningMode::Manual => *spawn_mode = SpawningMode::Automatic,
            SpawningMode::Automatic => *spawn_mode = SpawningMode::Manual,
        }
    }

    // Tab => toggle help menu.
    if keyboard.just_pressed(KeyCode::Tab) {
        for mut visibility in text_menus.iter_mut() {
            *visibility = match *visibility {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            };
        }
    }

    let mut camera_rig = camera.single_mut();

    // +/- => zoom camera.
    if keyboard.just_pressed(KeyCode::NumpadSubtract) || keyboard.just_pressed(KeyCode::Minus) {
        camera_rig.distance += MAX_CAMERA_DISTANCE / 15.0;
        camera_rig.distance = camera_rig
            .distance
            .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
    }

    if keyboard.just_pressed(KeyCode::NumpadAdd) {
        camera_rig.distance -= MAX_CAMERA_DISTANCE / 15.0;
        camera_rig.distance = camera_rig
            .distance
            .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
    }

    // Arrows => Move camera focus
    let left = keyboard.just_pressed(KeyCode::ArrowLeft);
    let right = keyboard.just_pressed(KeyCode::ArrowRight);

    if left || right {
        let mut closest = 0;
        let mut closest_distance = f32::MAX;
        for (i, (_, position)) in shapes.0.iter().enumerate() {
            let distance = camera_rig.target.distance(*position);
            if distance < closest_distance {
                closest = i;
                closest_distance = distance;
            }
        }
        if closest > 0 && left {
            camera_rig.target = shapes.0[closest - 1].1;
        }
        if closest < shapes.0.len() - 1 && right {
            camera_rig.target = shapes.0[closest + 1].1;
        }
    }
}

// Handle user mouse input for panning the camera around:
fn handle_mouse(
    mut button_events: EventReader<MouseButtonInput>,
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    mut camera: Query<&mut CameraRig>,
    mut mouse_pressed: ResMut<MousePressed>,
) {
    // Store left-pressed state in the MousePressed resource
    for button_event in button_events.read() {
        if button_event.button != MouseButton::Left {
            continue;
        }
        *mouse_pressed = MousePressed(button_event.state.is_pressed());
    }

    let mut camera_rig = camera.single_mut();

    let mouse_scroll = scroll_events
        .read()
        .fold(0.0, |acc, scroll_event| acc + scroll_event.y);
    camera_rig.distance -= mouse_scroll / 15.0 * MAX_CAMERA_DISTANCE;
    camera_rig.distance = camera_rig
        .distance
        .clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);

    // If the mouse is not pressed, just ignore motion events
    if !mouse_pressed.0 {
        return;
    }
    let displacement = motion_events
        .read()
        .fold(Vec2::ZERO, |acc, mouse_motion| acc + mouse_motion.delta);
    camera_rig.yaw += displacement.x / 90.;
    camera_rig.pitch += displacement.y / 90.;
    // The extra 0.01 is to disallow weird behaviour at the poles of the rotation
    camera_rig.pitch = camera_rig.pitch.clamp(-PI / 2.01, PI / 2.01);
}

#[allow(clippy::too_many_arguments)]
fn spawn_points(
    mut commands: Commands,
    mode: ResMut<SamplingMode>,
    shapes: Res<SampledShapes>,
    mut random_source: ResMut<RandomSource>,
    sample_mesh: Res<PointMesh>,
    sample_material: Res<PointMaterial>,
    mut spawn_queue: ResMut<SpawnQueue>,
    mut counter: ResMut<PointCounter>,
    spawn_mode: ResMut<SpawningMode>,
) {
    if let SpawningMode::Automatic = *spawn_mode {
        spawn_queue.0 += POINTS_PER_FRAME;
    }

    if spawn_queue.0 == 0 {
        return;
    }

    let rng = &mut random_source.0;

    // Don't go crazy
    for _ in 0..1000 {
        if spawn_queue.0 == 0 {
            break;
        }
        spawn_queue.0 -= 1;
        counter.0 += 1;

        let (shape, offset) = shapes.0.choose(rng).expect("There is at least one shape");

        // Get a single random Vec3:
        let sample: Vec3 = *offset
            + match *mode {
                SamplingMode::Interior => shape.sample_interior(rng),
                SamplingMode::Boundary => shape.sample_boundary(rng),
            };

        // Spawn a sphere at the random location:
        commands.spawn((
            PbrBundle {
                mesh: sample_mesh.0.clone(),
                material: match *mode {
                    SamplingMode::Interior => sample_material.interior.clone(),
                    SamplingMode::Boundary => sample_material.boundary.clone(),
                },
                transform: Transform::from_translation(sample).with_scale(Vec3::ZERO),
                ..default()
            },
            SamplePoint,
            SpawningPoint { progress: 0.0 },
        ));
    }
}

fn despawn_points(
    mut commands: Commands,
    samples: Query<Entity, With<SamplePoint>>,
    spawn_mode: Res<SpawningMode>,
    mut counter: ResMut<PointCounter>,
    mut random_source: ResMut<RandomSource>,
) {
    // Do not despawn automatically in manual mode
    if let SpawningMode::Manual = *spawn_mode {
        return;
    }

    if counter.0 < MAX_POINTS {
        return;
    }

    let rng = &mut random_source.0;
    // Skip a random amount of points to ensure random despawning
    let skip = rng.gen_range(0..counter.0);
    let despawn_amount = (counter.0 - MAX_POINTS).min(100);
    counter.0 -= samples
        .iter()
        .skip(skip)
        .take(despawn_amount)
        .map(|entity| {
            commands
                .entity(entity)
                .insert(DespawningPoint { progress: 0.0 })
                .remove::<SpawningPoint>()
                .remove::<SamplePoint>();
        })
        .count();
}

fn animate_spawning(
    mut commands: Commands,
    time: Res<Time>,
    mut samples: Query<(Entity, &mut Transform, &mut SpawningPoint)>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut point) in samples.iter_mut() {
        point.progress += dt / ANIMATION_TIME;
        transform.scale = Vec3::splat(point.progress.min(1.0));
        if point.progress >= 1.0 {
            commands.entity(entity).remove::<SpawningPoint>();
        }
    }
}

fn animate_despawning(
    mut commands: Commands,
    time: Res<Time>,
    mut samples: Query<(Entity, &mut Transform, &mut DespawningPoint)>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut point) in samples.iter_mut() {
        point.progress += dt / ANIMATION_TIME;
        // If the point is already smaller than expected, jump ahead with the despawning progress to avoid sudden jumps in size
        point.progress = f32::max(point.progress, 1.0 - transform.scale.x);
        transform.scale = Vec3::splat((1.0 - point.progress).max(0.0));
        if point.progress >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_camera(mut camera: Query<(&mut Transform, &CameraRig), Changed<CameraRig>>) {
    for (mut transform, rig) in camera.iter_mut() {
        let looking_direction =
            Quat::from_rotation_y(-rig.yaw) * Quat::from_rotation_x(rig.pitch) * Vec3::Z;
        transform.translation = rig.target - rig.distance * looking_direction;
        transform.look_at(rig.target, Dir3::Y);
    }
}

fn update_lights(
    mut lights: Query<&mut PointLight, With<FireflyLights>>,
    counter: Res<PointCounter>,
) {
    let saturation = (counter.0 as f32 / MAX_POINTS as f32).min(2.0);
    let intensity = 40_000.0 * saturation;
    for mut light in lights.iter_mut() {
        light.intensity = light.intensity.lerp(intensity, 0.04);
    }
}
