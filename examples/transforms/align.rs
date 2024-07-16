//! This example shows how to align the orientations of objects in 3D space along two axes using the `Transform::align` API.

use bevy::color::palettes::basic::{GRAY, RED, WHITE};
use bevy::input::mouse::{MouseButtonInput, MouseMotion};
use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (draw_ship_axes, draw_random_axes))
        .add_systems(Update, (handle_keypress, handle_mouse, rotate_ship).chain())
        .run();
}

/// This struct stores metadata for a single rotational move of the ship
#[derive(Component, Default)]
struct Ship {
    /// The initial transform of the ship move, the starting point of interpolation
    initial_transform: Transform,

    /// The target transform of the ship move, the endpoint of interpolation
    target_transform: Transform,

    /// The progress of the ship move in percentage points
    progress: u16,

    /// Whether the ship is currently in motion; allows motion to be paused
    in_motion: bool,
}

#[derive(Component)]
struct RandomAxes(Dir3, Dir3);

#[derive(Component)]
struct Instructions;

#[derive(Resource)]
struct MousePressed(bool);

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

// Setup

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);

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
    let first = seeded_rng.gen();
    let second = seeded_rng.gen();
    commands.spawn(RandomAxes(first, second));

    // Finally, our ship that is going to rotate
    commands.spawn((
        SceneBundle {
            scene: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/ship/craft_speederD.gltf")),
            ..default()
        },
        Ship {
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
            Press 'T' to align the ship to those directions.\n\
            Click and drag the mouse to rotate the camera.\n\
            Press 'H' to hide/show these instructions.",
            TextStyle::default(),
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

// Draw the main and secondary axes on the rotating ship
fn draw_ship_axes(mut gizmos: Gizmos, query: Query<&Transform, With<Ship>>) {
    let ship_transform = query.single();

    // Local Z-axis arrow, negative direction
    let z_ends = arrow_ends(ship_transform, Vec3::NEG_Z, 1.5);
    gizmos.arrow(z_ends.0, z_ends.1, RED);

    // local X-axis arrow
    let x_ends = arrow_ends(ship_transform, Vec3::X, 1.5);
    gizmos.arrow(x_ends.0, x_ends.1, Color::srgb(0.65, 0., 0.));
}

// Draw the randomly generated axes
fn draw_random_axes(mut gizmos: Gizmos, query: Query<&RandomAxes>) {
    let RandomAxes(v1, v2) = query.single();
    gizmos.arrow(Vec3::ZERO, 1.5 * *v1, WHITE);
    gizmos.arrow(Vec3::ZERO, 1.5 * *v2, GRAY);
}

// Actually update the ship's transform according to its initial source and target
fn rotate_ship(mut ship: Query<(&mut Ship, &mut Transform)>) {
    let (mut ship, mut ship_transform) = ship.single_mut();

    if !ship.in_motion {
        return;
    }

    let start = ship.initial_transform.rotation;
    let end = ship.target_transform.rotation;

    let p: f32 = ship.progress.into();
    let t = p / 100.;

    *ship_transform = Transform::from_rotation(start.slerp(end, t));

    if ship.progress == 100 {
        ship.in_motion = false;
    } else {
        ship.progress += 1;
    }
}

// Handle user inputs from the keyboard for dynamically altering the scenario
fn handle_keypress(
    mut ship: Query<(&mut Ship, &Transform)>,
    mut random_axes: Query<&mut RandomAxes>,
    mut instructions: Query<&mut Visibility, With<Instructions>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut seeded_rng: ResMut<SeededRng>,
) {
    let (mut ship, ship_transform) = ship.single_mut();
    let mut random_axes = random_axes.single_mut();

    if keyboard.just_pressed(KeyCode::KeyR) {
        // Randomize the target axes
        let first = seeded_rng.0.gen();
        let second = seeded_rng.0.gen();
        *random_axes = RandomAxes(first, second);

        // Stop the ship and set it up to transform from its present orientation to the new one
        ship.in_motion = false;
        ship.initial_transform = *ship_transform;
        ship.target_transform = random_axes_target_alignment(&random_axes);
        ship.progress = 0;
    }

    if keyboard.just_pressed(KeyCode::KeyT) {
        ship.in_motion ^= true;
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

// This is where `Transform::align` is actually used!
// Note that the choice of `Vec3::X` and `Vec3::Y` here matches the use of those in `draw_ship_axes`.
fn random_axes_target_alignment(random_axes: &RandomAxes) -> Transform {
    let RandomAxes(first, second) = random_axes;
    Transform::IDENTITY.aligned_by(Vec3::NEG_Z, *first, Vec3::X, *second)
}
