//! This example showcases a screen shake
//!
//! ## Controls
//!
//! | Key Binding  | Action               |
//! |:-------------|:---------------------|
//! | Space        | Trigger screen shake |

use bevy::{
    core_pipeline::bloom::Bloom,
    math::vec3,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const INTENSITY: f32 = 10.0;
const DURATION: f32 = 1.0;

#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_scene, setup_instructions, setup_camera))
        .add_systems(Update, (screen_shake, trigger_shake_on_space))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // World where we put the player
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(1000., 700.))),
        material: materials.add(Color::srgb(0.2, 0.2, 0.3)),
        ..default()
    });

    // Player
    commands.spawn((
        Player,
        MaterialMesh2dBundle {
            mesh: meshes.add(Circle::new(25.)).into(),
            material: materials.add(Color::srgb(6.25, 9.4, 9.1)), // RGB values exceed 1 to achieve a bright color for the bloom effect
            transform: Transform {
                translation: vec3(0., 0., 2.),
                ..default()
            },
            ..default()
        },
    ));

    // Add screen shake component to track shake duration and intensity
    commands.insert_resource(ScreenShake::new(10.0, 1.0));
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn(
        TextBundle::from_section("Use space to trigger a screen shake", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            }),
    );
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true, // HDR is required for the bloom effect
                ..default()
            },
            ..default()
        },
        Bloom::NATURAL,
    ));
}

#[derive(Resource)]
struct ScreenShake {
    intensity: f32,
    duration: f32,
    timer: Timer,
}

impl ScreenShake {
    fn new(intensity: f32, duration: f32) -> Self {
        ScreenShake {
            intensity,
            duration,
            timer: Timer::from_seconds(0.0, TimerMode::Once), // Start timer at 0
        }
    }

    fn start_shake(&mut self, intensity: f32, duration: f32) {
        self.intensity = intensity;
        self.duration = duration;
        self.timer = Timer::from_seconds(duration, TimerMode::Once); // Reset the timer
    }
}

fn trigger_shake_on_space(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut screen_shake: ResMut<ScreenShake>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        // Start the screen shake with intensity 10.0 for 1 second
        screen_shake.start_shake(INTENSITY, DURATION);
    }
}

fn screen_shake(
    time: Res<Time>,
    mut screen_shake: ResMut<ScreenShake>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    if screen_shake.timer.finished() {
        return; // No shake if the timer has finished
    }

    // Update the shake timer
    screen_shake.timer.tick(time.delta());

    // Calculate the progress of the shake (percentage of time passed)
    let elapsed = screen_shake.timer.elapsed_secs();
    let total_duration = screen_shake.timer.duration().as_secs_f32();
    let shake_progress = elapsed / total_duration;

    let mut rng = ChaCha8Rng::from_seed(42);
    let shake_amount = screen_shake.intensity * (1.0 - shake_progress);

    // Check if shake_amount is greater than zero to prevent invalid range error
    if shake_amount > 0.0 {
        for mut transform in query.iter_mut() {
            transform.translation.x = rng.gen_range(-shake_amount..shake_amount);
            transform.translation.y = rng.gen_range(-shake_amount..shake_amount);
        }
    }
}
