//! This example shows how to align the orientations of objects in 3D space along two axes using the `Transform::align` API.

use bevy::{
    color::{
        palettes::basic::{GRAY, RED, WHITE},
        Color,
    },
    input::mouse::{MouseButton, MouseButtonInput, MouseMotion},
    prelude::*,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_cube_axes, draw_random_axes))
        .add_systems(Update, (handle_keypress, handle_mouse, rotate_cube).chain())
        .run();
}

/// This struct stores metadata for a single rotational move of the cube
#[derive(Component, Default)]
struct Cube {
    /// The initial transform of the cube move, the starting point of interpolation
    initial_transform: Transform,

    /// The target transform of the cube move, the endpoint of interpolation
    target_transform: Transform,

    /// The progress of the cube move in percentage points
    progress: u16,

    /// Whether the cube is currently in motion; allows motion to be paused
    in_motion: bool,
}

#[derive(Component)]
struct RandomAxes(Vec3, Vec3);

#[derive(Component)]
struct Instructions;

#[derive(Resource)]
struct MousePressed(bool);

#[derive(Resource)]
struct SeededRng(StdRng);

// Setup

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut seeded_rng = StdRng::seed_from_u64(19878367467712);

    // A camera looking at the origin
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3., 2.5, 4.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // A plane that we can sit on top of
    commands.spawn(PbrBundle {
        transform: Transform::from_xyz(0., -2., 0.),
        mesh: meshes.add(Plane3d::default().mesh().size(100.0, 100.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // A light source
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 7.0, -4.0),
        ..default()
    });

    // Initialize random axes
    let first = random_direction(&mut seeded_rng);
    let second = random_direction(&mut seeded_rng);
    commands.spawn(RandomAxes(first, second));

    // Finally, our cube that is going to rotate
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb(0.5, 0.5, 0.5)),
            ..default()
        },
        Cube {
            initial_transform: Transform::IDENTITY,
            target_transform: random_axes_target_alignment(&RandomAxes(first, second)),
            ..default()
        },
    ));

    // Instructions for the example
    commands.spawn((
        TextBundle::from_section(
            "The bright red axis is the primary alignment axis, and it will always be\n\
            made to coincide with the primary target direction (white) exactly.\n\
            The fainter red axis is the secondary alignment axis, and it is made to\n\
            line up with the secondary target direction (gray) as closely as possible.\n\
            Press 'R' to generate random target directions.\n\
            Press 'T' to align the cube to those directions.\n\
            Click and drag the mouse to rotate the camera.\n\
            Press 'H' to hide/show these instructions.",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
        Instructions,
    ));

    commands.insert_resource(MousePressed(false));
    commands.insert_resource(SeededRng(seeded_rng));
}

// Update systems

// Draw the main and secondary axes on the rotating cube
fn draw_cube_axes(mut gizmos: Gizmos, query: Query<&Transform, With<Cube>>) {
    let cube_transform = query.single();

    // Local X-axis arrow
    let x_ends = arrow_ends(cube_transform, Vec3::X, 1.5);
    gizmos.arrow(x_ends.0, x_ends.1, RED);

    // local Y-axis arrow
    let y_ends = arrow_ends(cube_transform, Vec3::Y, 1.5);
    gizmos.arrow(y_ends.0, y_ends.1, Color::srgb(0.65, 0., 0.));
}

// Draw the randomly generated axes
fn draw_random_axes(mut gizmos: Gizmos, query: Query<&RandomAxes>) {
    let RandomAxes(v1, v2) = query.single();
    gizmos.arrow(Vec3::ZERO, 1.5 * *v1, WHITE);
    gizmos.arrow(Vec3::ZERO, 1.5 * *v2, GRAY);
}

// Actually update the cube's transform according to its initial source and target
fn rotate_cube(mut cube: Query<(&mut Cube, &mut Transform)>) {
    let (mut cube, mut cube_transform) = cube.single_mut();

    if !cube.in_motion {
        return;
    }

    let start = cube.initial_transform.rotation;
    let end = cube.target_transform.rotation;

    let p: f32 = cube.progress.into();
    let t = p / 100.;

    *cube_transform = Transform::from_rotation(start.slerp(end, t));

    if cube.progress == 100 {
        cube.in_motion = false;
    } else {
        cube.progress += 1;
    }
}

// Handle user inputs from the keyboard for dynamically altering the scenario
fn handle_keypress(
    mut cube: Query<(&mut Cube, &Transform)>,
    mut random_axes: Query<&mut RandomAxes>,
    mut instructions: Query<&mut Visibility, With<Instructions>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut seeded_rng: ResMut<SeededRng>,
) {
    let (mut cube, cube_transform) = cube.single_mut();
    let mut random_axes = random_axes.single_mut();

    if keyboard.just_pressed(KeyCode::KeyR) {
        // Randomize the target axes
        let first = random_direction(&mut seeded_rng.0);
        let second = random_direction(&mut seeded_rng.0);
        *random_axes = RandomAxes(first, second);

        // Stop the cube and set it up to transform from its present orientation to the new one
        cube.in_motion = false;
        cube.initial_transform = *cube_transform;
        cube.target_transform = random_axes_target_alignment(&random_axes);
        cube.progress = 0;
    }

    if keyboard.just_pressed(KeyCode::KeyT) {
        cube.in_motion ^= true;
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        let mut instructions_viz = instructions.single_mut();
        if *instructions_viz == Visibility::Hidden {
            *instructions_viz = Visibility::Visible;
        } else {
            *instructions_viz = Visibility::Hidden;
        }
    }
}

// Handle user mouse input for panning the camera around
fn handle_mouse(
    mut button_events: EventReader<MouseButtonInput>,
    mut motion_events: EventReader<MouseMotion>,
    mut camera: Query<&mut Transform, With<Camera>>,
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
    let displacement = motion_events
        .read()
        .fold(0., |acc, mouse_motion| acc + mouse_motion.delta.x);
    let mut camera_transform = camera.single_mut();
    camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(-displacement / 75.));
}

// Helper functions (i.e. non-system functions)

fn arrow_ends(transform: &Transform, axis: Vec3, length: f32) -> (Vec3, Vec3) {
    let local_vector = length * (transform.rotation * axis);
    (transform.translation, transform.translation + local_vector)
}

fn random_direction(rng: &mut impl Rng) -> Vec3 {
    let height = rng.gen::<f32>() * 2. - 1.;
    let theta = rng.gen::<f32>() * 2. * PI;

    build_direction(height, theta)
}

fn build_direction(height: f32, theta: f32) -> Vec3 {
    let z = height;
    let m = f32::acos(z).sin();
    let x = theta.cos() * m;
    let y = theta.sin() * m;

    Vec3::new(x, y, z)
}

// This is where `Transform::align` is actually used!
// Note that the choice of `Vec3::X` and `Vec3::Y` here matches the use of those in `draw_cube_axes`.
fn random_axes_target_alignment(random_axes: &RandomAxes) -> Transform {
    let RandomAxes(first, second) = random_axes;
    Transform::IDENTITY.aligned_by(Vec3::X, *first, Vec3::Y, *second)
}
