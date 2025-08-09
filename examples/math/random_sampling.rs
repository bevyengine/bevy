//! This example shows how to sample random points from primitive shapes.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, MouseButtonInput},
    math::prelude::*,
    mesh::SphereKind,
    prelude::*,
};
use rand::{distr::Distribution, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_mouse, handle_keypress))
        .run();
}

/// Resource for the random sampling mode, telling whether to sample the interior or the boundary.
#[derive(Resource)]
enum Mode {
    Interior,
    Boundary,
}

/// Resource storing the shape being sampled.
#[derive(Resource)]
struct SampledShape(Cuboid);

/// The source of randomness used by this example.
#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

/// A container for the handle storing the mesh used to display sampled points as spheres.
#[derive(Resource)]
struct PointMesh(Handle<Mesh>);

/// A container for the handle storing the material used to display sampled points.
#[derive(Resource)]
struct PointMaterial(Handle<StandardMaterial>);

/// Marker component for sampled points.
#[derive(Component)]
struct SamplePoint;

/// The pressed state of the mouse, used for camera motion.
#[derive(Resource)]
struct MousePressed(bool);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Use seeded rng and store it in a resource; this makes the random output reproducible.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(RandomSource(seeded_rng));

    // Make a plane for establishing space.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -2.5, 0.0),
    ));

    // Store the shape we sample from in a resource:
    let shape = Cuboid::from_length(2.9);
    commands.insert_resource(SampledShape(shape));

    // The sampled shape shown transparently:
    commands.spawn((
        Mesh3d(meshes.add(shape)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.2, 0.1, 0.6, 0.3),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
    ));

    // A light:
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // A camera:
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Store the mesh and material for sample points in resources:
    commands.insert_resource(PointMesh(
        meshes.add(
            Sphere::new(0.03)
                .mesh()
                .kind(SphereKind::Ico { subdivisions: 3 }),
        ),
    ));
    commands.insert_resource(PointMaterial(materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.8),
        metallic: 0.8,
        ..default()
    })));

    // Instructions for the example:
    commands.spawn((
        Text::new(
            "Controls:\n\
            M: Toggle between sampling boundary and interior.\n\
            R: Restart (erase all samples).\n\
            S: Add one random sample.\n\
            D: Add 100 random samples.\n\
            Rotate camera by holding left mouse and panning left/right.",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // The mode starts with interior points.
    commands.insert_resource(Mode::Interior);

    // Starting mouse-pressed state is false.
    commands.insert_resource(MousePressed(false));
}

// Handle user inputs from the keyboard:
fn handle_keypress(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<Mode>,
    shape: Res<SampledShape>,
    mut random_source: ResMut<RandomSource>,
    sample_mesh: Res<PointMesh>,
    sample_material: Res<PointMaterial>,
    samples: Query<Entity, With<SamplePoint>>,
) {
    // R => restart, deleting all samples
    if keyboard.just_pressed(KeyCode::KeyR) {
        for entity in &samples {
            commands.entity(entity).despawn();
        }
    }

    // S => sample once
    if keyboard.just_pressed(KeyCode::KeyS) {
        let rng = &mut random_source.0;

        // Get a single random Vec3:
        let sample: Vec3 = match *mode {
            Mode::Interior => shape.0.sample_interior(rng),
            Mode::Boundary => shape.0.sample_boundary(rng),
        };

        // Spawn a sphere at the random location:
        commands.spawn((
            Mesh3d(sample_mesh.0.clone()),
            MeshMaterial3d(sample_material.0.clone()),
            Transform::from_translation(sample),
            SamplePoint,
        ));

        // NOTE: The point is inside the cube created at setup just because of how the
        // scene is constructed; in general, you would want to use something like
        // `cube_transform.transform_point(sample)` to get the position of where the sample
        // would be after adjusting for the position and orientation of the cube.
        //
        // If the spawned point also needed to follow the position of the cube as it moved,
        // then making it a child entity of the cube would be a good approach.
    }

    // D => generate many samples
    if keyboard.just_pressed(KeyCode::KeyD) {
        let mut rng = &mut random_source.0;

        // Get 100 random Vec3s:
        let samples: Vec<Vec3> = match *mode {
            Mode::Interior => {
                let dist = shape.0.interior_dist();
                dist.sample_iter(&mut rng).take(100).collect()
            }
            Mode::Boundary => {
                let dist = shape.0.boundary_dist();
                dist.sample_iter(&mut rng).take(100).collect()
            }
        };

        // For each sample point, spawn a sphere:
        for sample in samples {
            commands.spawn((
                Mesh3d(sample_mesh.0.clone()),
                MeshMaterial3d(sample_material.0.clone()),
                Transform::from_translation(sample),
                SamplePoint,
            ));
        }

        // NOTE: See the previous note above regarding the positioning of these samples
        // relative to the transform of the cube containing them.
    }

    // M => toggle mode between interior and boundary.
    if keyboard.just_pressed(KeyCode::KeyM) {
        match *mode {
            Mode::Interior => *mode = Mode::Boundary,
            Mode::Boundary => *mode = Mode::Interior,
        }
    }
}

// Handle user mouse input for panning the camera around:
fn handle_mouse(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut button_events: EventReader<MouseButtonInput>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
    mut mouse_pressed: ResMut<MousePressed>,
) {
    // Store left-pressed state in the MousePressed resource
    for button_event in button_events.read() {
        if button_event.button != MouseButton::Left {
            continue;
        }
        *mouse_pressed = MousePressed(button_event.state.is_pressed());
    }

    // If the mouse is not pressed, just ignore motion events
    if !mouse_pressed.0 {
        return;
    }
    if accumulated_mouse_motion.delta != Vec2::ZERO {
        let displacement = accumulated_mouse_motion.delta.x;
        camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(-displacement / 150.));
    }
}
