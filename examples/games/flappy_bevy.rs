//! A simplified Flappy Bird but with many birds

use bevy::prelude::*;

use bevy::sprite::collide_aabb::collide;
use bevy::window::PresentMode;

use rand::Rng;

const CHUNK_SIZE: f32 = 300.0;
const CAMERA_SPEED: f32 = 120.0;
const GAME_HEIGHT: f32 = 500.0;
const SCREEN_HEIGHT: f32 = 1500.0;
const CLEANUP_X_DIST: f32 = 1500.0;
const CHAOS_AMOUNT_X: f32 = 250.0;
const CHAOS_AMOUNT_Y: f32 = 600.0;
const DRIFT_TO_CENTER_AMOUNT: f32 = 0.01;
const FLAP_STRENGTH: f32 = 250.0;
const BIRD_SIZE: f32 = 24.0;
const BIRD_REPRODUCTION_CHANCE: f32 = 1.0;
const MAX_BIRDS: usize = 500;
const GRAVITY: f32 = 400.0;

fn randf() -> f32 {
    rand::thread_rng().gen::<f32>()
}

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
    .add_startup_system(spawn_camera)
    .add_startup_system(load_art)
    .add_startup_system(bird_startup)
    .add_system(spawn_bird)
    .add_system(bird_collision)
    .add_system(bird_reproduction)
    .add_system(bird_control.after(gravity))
    .add_system(terrain_gen)
    .add_system(advance_camera)
    .add_system(brownian_drift)
    .add_system(velocity)
    .add_system(gravity)
    .add_system(terrain_cleanup)
    .add_system(drift_to_center)
    .add_event::<GenerateChunk>()
    .add_event::<SpawnBird>();

    app.run();
}

pub struct Art {
    bird_icon: Handle<Image>,
}

fn load_art(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Art {
        bird_icon: asset_server.load("branding/icon.png"),
    });
}
struct GenerateChunk {
    new_chunk_index: i32,
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
        if chunk_index != new_chunk_index {
            outgoing_generate_chunk_events.send(GenerateChunk {
                new_chunk_index: new_chunk_index + 3,
            });
        }
    }
}

#[derive(Component)]
struct Obstacle;

fn terrain_cleanup(
    mut commands: Commands,
    q: Query<(Entity, &Transform), With<Obstacle>>,
    cam: Query<&Transform, With<Camera>>,
) {
    let cam_x = cam.single().translation.x;
    for (e, t) in q.iter() {
        if t.translation.x < cam_x - CLEANUP_X_DIST {
            commands.entity(e).despawn();
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
        let gap_y_pos = GAME_HEIGHT * (randf() - 0.5);
        let pillar_width = 50.0 + 110.0 * randf();
        // make the gap no narrower than the pillar is wide
        let gap_size = (50.0 + 250.0 * randf()).max(pillar_width);
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

fn spawn_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

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

/// Event that causes a new bird to spawn
struct SpawnBird {
    new_bird_pos: Vec2,
    new_bird_velocity: Vec2,
}

fn bird_startup(mut spawn_bird_events: EventWriter<SpawnBird>) {
    spawn_bird_events.send(SpawnBird {
        new_bird_pos: Vec2::ZERO,
        new_bird_velocity: Vec2::new(CAMERA_SPEED, 0.0),
    });
}

fn spawn_bird(
    mut commands: Commands,
    mut spawn_bird_events: EventReader<SpawnBird>,
    art: Res<Art>,
) {
    for ev in spawn_bird_events.iter() {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(randf(), randf(), randf()),
                    custom_size: Some(Vec2::splat(BIRD_SIZE)),
                    ..default()
                },
                texture: art.bird_icon.clone(),
                transform: Transform::from_translation(ev.new_bird_pos.extend(randf() * 100.0)),
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

fn bird_collision(
    mut commands: Commands,
    birds: Query<(Entity, &Sprite, &Transform), With<Bird>>,
    obstacles: Query<(&Sprite, &Transform), With<Obstacle>>,
) {
    for (bird_entity, bird_sprite, bird_transform) in birds.iter() {
        for (obstacle_sprite, obstacle_transform) in obstacles.iter() {
            let collision_result = collide(
                bird_transform.translation,
                bird_sprite.custom_size.unwrap(),
                obstacle_transform.translation,
                obstacle_sprite.custom_size.unwrap(),
            );
            if collision_result.is_some() {
                commands.entity(bird_entity).despawn();
            }
        }
    }
}

fn bird_reproduction(
    q: Query<(&Transform, &Velocity), With<Bird>>,
    time: Res<Time>,
    mut spawn_bird_events: EventWriter<SpawnBird>,
) {
    let mut bird_count = 0;
    for (_t, _v) in q.iter() {
        bird_count += 1;
    }
    for (t, v) in q.iter() {
        if bird_count < MAX_BIRDS && randf() < BIRD_REPRODUCTION_CHANCE * time.delta().as_secs_f32()
        {
            spawn_bird_events.send(SpawnBird {
                new_bird_pos: t.translation.truncate(),
                new_bird_velocity: v.velocity,
            });
        }
    }
}

fn brownian_drift(mut q: Query<&mut Velocity, With<BrownianDrift>>, time: Res<Time>) {
    for mut v in q.iter_mut() {
        v.velocity += Vec2::new(
            (randf() - 0.5) * CHAOS_AMOUNT_X,
            (randf() - 0.5) * CHAOS_AMOUNT_Y,
        ) * time.delta().as_secs_f32();
    }
}

fn bird_control(input: Res<Input<KeyCode>>, mut birds: Query<&mut Velocity, With<Bird>>) {
    if input.just_pressed(KeyCode::Space) {
        for mut v in birds.iter_mut() {
            v.velocity.y = FLAP_STRENGTH;
        }
    }
}
