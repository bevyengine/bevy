//! A simplified Flappy Bird but with many birds. Press space to flap.

use bevy::prelude::*;

use bevy::sprite::collide_aabb::{collide, Collision};
use bevy::window::PresentMode;

use rand::random;

const CHUNK_SIZE: f32 = 300.0;
const CAMERA_SPEED: f32 = 120.0;
const GAME_HEIGHT: f32 = 500.0;
const SCREEN_HEIGHT: f32 = 1500.0;
const CLEANUP_X_DIST: f32 = 1500.0;
const BROWNIAN_DRIFT_AMOUNT_X: f32 = 250.0;
const BROWNIAN_DRIFT_AMOUNT_Y: f32 = 600.0;
const DRIFT_TO_CENTER_AMOUNT: f32 = 0.05;
const FLAP_STRENGTH: f32 = 240.0;
const BIRD_SIZE: f32 = 24.0;
const BIRD_REPRODUCTION_CHANCE: f32 = 1.0;
const MAX_BIRDS: usize = 500;
const GRAVITY: f32 = 400.0;
const GAP_VARIABILITY: f32 = 0.9;
const AUTO_FLAP_INTERVAL_SECS: f32 = 1.1;
const MOURN_TIME_SECS: f32 = 0.5;

pub fn main() {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        width: 1600.,
        height: 900.,
        title: "Flappy Bevy".to_string(),
        present_mode: PresentMode::Immediate, // smooth but power hungry
        resizable: true,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins)
    .add_event::<Flap>()
    .add_event::<GenerateChunk>()
    .add_event::<SpawnBird>()
    .insert_resource(AutoFlapState::default())
    .insert_resource(MourningState::default())
    .add_startup_system(load_art)
    .add_startup_system(bird_startup) // generates a SpawnBird event
    .add_system(spawn_bird) // responds to SpawnBird events
    .add_system(input) // generates Flap events
    .add_system_to_stage(CoreStage::PostUpdate, flap) // responds to Flap events, ordering prevents physics bug
    .add_system(auto_flap) // play the game automatically at the start
    .add_system(bird_collision) // despawn birds that collide with pillars or floor
    .add_system(bird_reproduction) // slowly grow the flock
    .add_system(brownian_drift) // make the flock drift apart
    .add_system(velocity) // integrates velocity over time, mutating translation
    .add_system(gravity) // makes gravity influence to velocity
    .add_system(drift_to_center) // gently return birds to (0, 0)
    .add_system(mourn.before(spawn_bird)) // respawn a bird when all die. Ordering necessary because counting birds
    .add_system(terrain_gen) // generate pillars off-screen to the right
    .add_system(terrain_cleanup) // remove pillars off-screen to the left
    .add_startup_system(spawn_camera)
    .add_system(advance_camera); // move the camera right at a constant speed

    app.run();
}

/// Event that causes a new bird to spawn
struct SpawnBird {
    new_bird_pos: Option<Vec2>,
    new_bird_velocity: Vec2,
}

/// Event that causes all birds on screen to flap
struct Flap;

/// Resource
struct Art {
    bird_icon: Handle<Image>,
}

/// Resource
struct AutoFlapState {
    active: bool,
    timer: Timer,
}

impl Default for AutoFlapState {
    fn default() -> AutoFlapState {
        AutoFlapState {
            active: true,
            timer: Timer::from_seconds(AUTO_FLAP_INTERVAL_SECS, true),
        }
    }
}

/// Resource
struct MourningState {
    active: bool,
    timer: Timer,
}

impl Default for MourningState {
    fn default() -> MourningState {
        MourningState {
            active: false,
            timer: Timer::from_seconds(MOURN_TIME_SECS, false),
        }
    }
}

struct GenerateChunk {
    new_chunk_index: i32,
}

#[derive(Component)]
struct Obstacle;

#[derive(Component, Default)]
struct Velocity {
    velocity: Vec2,
}

#[derive(Component)]
struct BrownianDrift;

#[derive(Component)]
struct Gravity;

#[derive(Component)]
struct Bird;

#[derive(Component)]
struct DriftToCenter;

fn load_art(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Art {
        bird_icon: asset_server.load("branding/icon.png"),
    });
}

fn bird_startup(mut spawn_bird_events: EventWriter<SpawnBird>) {
    spawn_bird_events.send(SpawnBird {
        new_bird_pos: Some(Vec2::ZERO),
        new_bird_velocity: Vec2::new(CAMERA_SPEED, 0.0),
    });
}

fn spawn_bird(
    mut commands: Commands,
    mut spawn_bird_events: EventReader<SpawnBird>,
    cam: Query<&Transform, With<Camera>>,
    art: Res<Art>,
) {
    for ev in spawn_bird_events.iter() {
        let camera_pos = cam.single().translation.truncate();
        // if a bird position is not supplied, spawn in the center of the view
        let new_bird_pos = ev.new_bird_pos.unwrap_or(camera_pos);
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(random::<f32>(), random::<f32>(), random::<f32>()),
                    custom_size: Some(Vec2::splat(BIRD_SIZE)),
                    ..default()
                },
                texture: art.bird_icon.clone(),
                transform: Transform::from_translation(
                    new_bird_pos.extend(random::<f32>() * 100.0),
                ),
                ..default()
            })
            .insert(Velocity {
                velocity: ev.new_bird_velocity,
            })
            .insert(Gravity)
            .insert(Bird)
            .insert(BrownianDrift)
            .insert(DriftToCenter);
    }
}

fn input(
    input: Res<Input<KeyCode>>,
    mut flap_events: EventWriter<Flap>,
    mut auto_flap_state: ResMut<AutoFlapState>,
) {
    if input.just_pressed(KeyCode::Space) {
        flap_events.send(Flap {});
        auto_flap_state.active = false;
    }
}

fn flap(flap_events: EventReader<Flap>, mut birds: Query<&mut Velocity, With<Bird>>) {
    if !flap_events.is_empty() {
        for mut v in birds.iter_mut() {
            v.velocity.y = FLAP_STRENGTH;
        }
    }
}

fn auto_flap(
    time: Res<Time>,
    mut flap_events: EventWriter<Flap>,
    mut auto_flap_state: ResMut<AutoFlapState>,
) {
    if !auto_flap_state.active {
        return;
    }
    auto_flap_state.timer.tick(time.delta());
    if auto_flap_state.timer.just_finished() {
        flap_events.send(Flap {});
    }
}

fn bird_collision(
    mut commands: Commands,
    birds: Query<(Entity, &Sprite, &Transform), With<Bird>>,
    obstacles: Query<(&Sprite, &Transform), With<Obstacle>>,
) {
    for (bird_entity, bird_sprite, bird_transform) in birds.iter() {
        let mut collision_result: Option<Collision> = None;
        for (obstacle_sprite, obstacle_transform) in obstacles.iter() {
            if collision_result.is_none() {
                collision_result = collide(
                    bird_transform.translation,
                    bird_sprite.custom_size.unwrap(),
                    obstacle_transform.translation,
                    obstacle_sprite.custom_size.unwrap(),
                );
            }
        }
        if collision_result.is_some() || bird_transform.translation.y < -SCREEN_HEIGHT * 0.5 {
            commands.entity(bird_entity).despawn();
        }
    }
}

fn bird_reproduction(
    q: Query<(&Transform, &Velocity), With<Bird>>,
    time: Res<Time>,
    mut spawn_bird_events: EventWriter<SpawnBird>,
) {
    let bird_count = q.iter().count();
    if bird_count < MAX_BIRDS {
        for (t, v) in q.iter() {
            if random::<f32>() < BIRD_REPRODUCTION_CHANCE * time.delta().as_secs_f32() {
                spawn_bird_events.send(SpawnBird {
                    new_bird_pos: Some(t.translation.truncate()),
                    new_bird_velocity: v.velocity,
                });
            }
        }
    }
}

fn brownian_drift(mut q: Query<&mut Velocity, With<BrownianDrift>>, time: Res<Time>) {
    for mut v in q.iter_mut() {
        v.velocity += Vec2::new(
            (random::<f32>() - 0.5) * BROWNIAN_DRIFT_AMOUNT_X,
            (random::<f32>() - 0.5) * BROWNIAN_DRIFT_AMOUNT_Y,
        ) * time.delta().as_secs_f32();
    }
}

fn velocity(mut q: Query<(&Velocity, &mut Transform)>, time: Res<Time>) {
    for (v, mut t) in q.iter_mut() {
        t.translation += (v.velocity * time.delta().as_secs_f32()).extend(0.0);
    }
}

fn gravity(mut q: Query<&mut Velocity, With<Gravity>>, time: Res<Time>) {
    for mut v in q.iter_mut() {
        v.velocity.y -= time.delta().as_secs_f32() * GRAVITY;
    }
}

/// The flock has a tendency to drift offscreen - gently bring it back to the center
fn drift_to_center(
    mut q: Query<(&mut Velocity, &Transform)>,
    cam: Query<&Transform, With<Camera>>,
    time: Res<Time>,
) {
    for (mut v, t) in q.iter_mut() {
        v.velocity.x -= (t.translation.x - cam.single().translation.x)
            * DRIFT_TO_CENTER_AMOUNT
            * time.delta().as_secs_f32();
    }
}

fn mourn(
    time: Res<Time>,
    mut mourning_state: ResMut<MourningState>,
    bird_q: Query<(), With<Bird>>,
    mut spawn_bird_events: EventWriter<SpawnBird>,
    mut auto_flap_state: ResMut<AutoFlapState>,
) {
    if bird_q.iter().count() == 0 && !mourning_state.active && time.seconds_since_startup() > 1.0 {
        // don't mourn on the first frame even though there are no birds
        mourning_state
            .timer
            .set_duration(std::time::Duration::from_secs(4));
        mourning_state.timer.reset();
        mourning_state.active = true;
    }
    if mourning_state.active {
        mourning_state.timer.tick(time.delta());
        if mourning_state.timer.just_finished() {
            mourning_state.active = false;
            spawn_bird_events.send(SpawnBird {
                new_bird_pos: None,
                new_bird_velocity: Vec2::new(CAMERA_SPEED, 0.0),
            });
            auto_flap_state.active = true;
        }
    }
}

fn terrain_gen(
    mut commands: Commands,
    mut incoming_generate_chunk_events: EventReader<GenerateChunk>,
) {
    for ev in incoming_generate_chunk_events.iter() {
        let x_pos = CHUNK_SIZE * ev.new_chunk_index as f32;
        // generate some terrain within x_pos..x_pos+width
        let gap_y_pos = GAME_HEIGHT * (random::<f32>() - 0.5) * GAP_VARIABILITY;
        let pillar_width = 50.0 + 110.0 * random::<f32>();
        // make the gap no narrower than the pillar is wide
        let gap_size = (65.0 + 250.0 * random::<f32>()).max(pillar_width);
        for (top_y_pos, bottom_y_pos) in [
            (-SCREEN_HEIGHT * 0.5, gap_y_pos - gap_size * 0.5),
            (gap_y_pos + gap_size * 0.5, SCREEN_HEIGHT * 0.5),
        ] {
            let pillar_origin = Vec2::new(x_pos, (top_y_pos + bottom_y_pos) * 0.5);
            let pillar_size = Vec2::new(pillar_width, bottom_y_pos - top_y_pos);
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.25, 0.25, 0.75),
                        custom_size: Some(pillar_size),
                        ..default()
                    },
                    transform: Transform::from_translation(pillar_origin.extend(0.0)),
                    ..default()
                })
                .insert(Obstacle);
        }
    }
}

fn terrain_cleanup(
    mut commands: Commands,
    q: Query<(Entity, &Transform), With<Obstacle>>,
    cam: Query<&Transform, With<Camera>>,
) {
    let cam_x = cam.single().translation.x;
    for (e, t) in q.iter() {
        // remove obstacles at the left
        if t.translation.x < cam_x - CLEANUP_X_DIST {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn advance_camera(
    mut q: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
    mut outgoing_generate_chunk_events: EventWriter<GenerateChunk>,
) {
    if let Ok(mut transform) = q.get_single_mut() {
        let chunk_index = (transform.translation.x / CHUNK_SIZE).floor() as i32;
        transform.translation.x += time.delta().as_secs_f32() * CAMERA_SPEED;
        let new_chunk_index = (transform.translation.x / CHUNK_SIZE).floor() as i32;
        // if the camera has moved over a chunk boundary
        if chunk_index != new_chunk_index {
            // generate a new chunk offscreen to the right
            outgoing_generate_chunk_events.send(GenerateChunk {
                new_chunk_index: new_chunk_index + 3,
            });
        }
    }
}
