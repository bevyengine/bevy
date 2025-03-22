//! This example showcases a 2D screen shake using concept in this video: `<https://www.youtube.com/watch?v=tu-Qe66AvtY>`
//!
//! ## Controls
//!
//! | Key Binding  | Action               |
//! |:-------------|:---------------------|
//! | Space        | Trigger screen shake |

use bevy::{prelude::*, render::camera::SubCameraView, sprite::MeshMaterial2d};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const CAMERA_DECAY_RATE: f32 = 0.9; // Adjust this for smoother or snappier decay
const TRAUMA_DECAY_SPEED: f32 = 0.5; // How fast trauma decays
const TRAUMA_INCREMENT: f32 = 1.0; // Increment of trauma per frame when holding space

// screen_shake parameters, maximum addition by frame not actual maximum overall values
const MAX_ANGLE: f32 = 0.5;
const MAX_OFFSET: f32 = 500.0;

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
    // World where we move the player
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1000., 700.))),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.2, 0.3))),
    ));

    // Player
    commands.spawn((
        Player,
        Mesh2d(meshes.add(Rectangle::new(50.0, 100.0))), // Rectangle size (width, height)
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.94, 0.91))), // RGB values must be in range 0.0 to 1.0
        Transform::from_xyz(0., 0., 2.),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50.0, 50.0))), // Rectangle size (width, height)
        MeshMaterial2d(materials.add(Color::srgb(0.85, 0.0, 0.2))), // RGB values must be in range 0.0 to 1.0
        Transform::from_xyz(-450.0, 200.0, 2.),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(70.0, 50.0))), // Rectangle size (width, height)
        MeshMaterial2d(materials.add(Color::srgb(0.5, 0.8, 0.2))), // RGB values must be in range 0.0 to 1.0
        Transform::from_xyz(450.0, -150.0, 2.),
    ));
    commands.init_resource::<ScreenShake>();
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        Text::new("Hold space to trigger a screen shake"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            sub_camera_view: Some(SubCameraView {
                full_size: UVec2::new(1000, 700),
                offset: Vec2::new(0.0, 0.0),
                size: UVec2::new(1000, 700),
            }),
            order: 1,
            ..default()
        },
    ));
}

#[derive(Resource, Clone)]
struct ScreenShake {
    max_angle: f32,
    max_offset: f32,
    trauma: f32,
    latest_position: Option<Vec2>,
}

impl Default for ScreenShake {
    fn default() -> Self {
        Self {
            max_angle: 0.0,
            max_offset: 0.0,
            trauma: 0.0,
            latest_position: Some(Vec2::default()),
        }
    }
}

impl ScreenShake {
    fn start_shake(&mut self, max_angle: f32, max_offset: f32, trauma: f32, final_position: Vec2) {
        self.max_angle = max_angle;
        self.max_offset = max_offset;
        self.trauma = trauma.clamp(0.0, 1.0);
        self.latest_position = Some(final_position);
    }
}

fn trigger_shake_on_space(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut screen_shake: ResMut<ScreenShake>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        let screen_shake_clone = screen_shake.clone();
        screen_shake.start_shake(
            MAX_ANGLE,
            MAX_OFFSET,
            screen_shake_clone.trauma + TRAUMA_INCREMENT * time.delta_secs(),
            Vec2 { x: 0.0, y: 0.0 },
        ); // final_position should be your current player position
    }
}

fn screen_shake(
    time: Res<Time>,
    mut screen_shake: ResMut<ScreenShake>,
    mut query: Query<(&mut Camera, &mut Transform)>,
) {
    let mut rng = ChaCha8Rng::from_entropy();
    let shake = screen_shake.trauma * screen_shake.trauma;
    let angle = (screen_shake.max_angle * shake).to_radians() * rng.gen_range(-1.0..1.0);
    let offset_x = screen_shake.max_offset * shake * rng.gen_range(-1.0..1.0);
    let offset_y = screen_shake.max_offset * shake * rng.gen_range(-1.0..1.0);

    if shake > 0.0 {
        for (mut camera, mut transform) in query.iter_mut() {
            // Position
            let sub_view = camera.sub_camera_view.as_mut().unwrap();
            let target = sub_view.offset
                + Vec2 {
                    x: offset_x,
                    y: offset_y,
                };
            sub_view
                .offset
                .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());

            // Rotation
            let rotation = Quat::from_rotation_z(angle);
            transform.rotation = transform
                .rotation
                .interpolate_stable(&(transform.rotation.mul_quat(rotation)), CAMERA_DECAY_RATE);
        }
    } else {
        // return camera to the latest position of player (it's fixed in this example case)
        if let Ok((mut camera, mut transform)) = query.single_mut() {
            let sub_view = camera.sub_camera_view.as_mut().unwrap();
            let target = screen_shake.latest_position.unwrap();
            sub_view
                .offset
                .smooth_nudge(&target, 1.0, time.delta_secs());
            transform.rotation = transform.rotation.interpolate_stable(&Quat::IDENTITY, 0.1);
        }
    }
    // Decay the trauma over time
    screen_shake.trauma -= TRAUMA_DECAY_SPEED * time.delta_secs();
    screen_shake.trauma = screen_shake.trauma.clamp(0.0, 1.0);
}
