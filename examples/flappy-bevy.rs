//! A simplified Flappy Bird but with many birds. Press space to flap.

use bevy::ecs::system::RunSystemOnce;
use bevy::math::bounding::{Aabb2d, BoundingCircle, IntersectsVolume};
use bevy::window::PrimaryWindow;
use bevy::{ecs::world::Command, prelude::*};

use rand::random;

const CAMERA_SPEED: f32 = 120.0;

#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Default)]
enum GameState {
    #[default]
    Loading,
    Over,
    Playing,
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            // TODO: remove
            resizable: false,
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_systems(Startup, load_assets);

    app.add_plugins((asset_plugin, bird_plugin, physics_plugin, terrain_plugin));
    app.configure_sets(
        FixedUpdate,
        AppSet::Physics.run_if(in_state(GameState::Playing)),
    );
    app.configure_sets(Update, AppSet::Loading.run_if(in_state(GameState::Loading)));
    app.configure_sets(
        Update,
        (AppSet::RecordInput, AppSet::Playing)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
    app.run();
}

// High-level groupings of systems for the game.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum AppSet {
    // Record player input.
    RecordInput,
    // Anything to do with gravity or velocity.
    Physics,
    // Asset loading updates.
    Loading,
    // In-game updates.
    Playing,
}

// Asset Plugin
//
// Normally, plugins would be separated into different modules. For the purposes of this example,
// everything is in one file but it's a valuable pattern to keep in mind. With the aid of `State`s
// and `SystemSet`s, we can often separate a Bevy app into areas of concern. It's not uncommon to
// nest plugins within plugins, so long as it makes sense for your overall design.
fn asset_plugin(app: &mut App) {
    app.add_systems(Startup, load_assets);
    app.add_systems(Update, wait_for_asset_load.in_set(AppSet::Loading));
}

#[derive(Resource)]
struct TextureAssets {
    bird: Handle<Image>,
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TextureAssets {
        bird: asset_server.load("branding/icon.png"),
    });
}

fn wait_for_asset_load(
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
    texture_assets: Res<TextureAssets>,
) {
    if asset_server.is_loaded_with_dependencies(&texture_assets.bird) {
        next_state.set(GameState::Playing);
    }
}

// Bird Plugin
fn bird_plugin(app: &mut App) {
    app.init_resource::<FlockSettings>();
    // This will run once we've finished loading assets and we know we're ready to go.
    app.add_systems(OnEnter(GameState::Playing), setup);
    app.add_systems(Update, input.in_set(AppSet::RecordInput));
    app.add_systems(Update, reproduction.in_set(AppSet::Playing));
}

#[derive(Component)]
struct Bird;

#[derive(Resource)]
struct FlockSettings {
    pub bird_size: f32,
    pub drift: Vec2,
    pub max_birds: usize,
    pub reproduction_chance: f32,
}

impl Default for FlockSettings {
    fn default() -> Self {
        Self {
            bird_size: 24.0,
            drift: Vec2::new(2.0, 2.0),
            max_birds: 500,
            reproduction_chance: 1.0,
        }
    }
}

struct SpawnBird {
    translation: Vec3,
    velocity: Vec2,
}

// This allows us to `queue` up a `Command` to spawn a `Bird`, with whatever configuration we might
// need to display it correctly.
impl Command for SpawnBird {
    fn apply(self, world: &mut World) {
        world.run_system_once_with(self, spawn_bird);
    }
}

fn spawn_bird(
    In(config): In<SpawnBird>,
    mut commands: Commands,
    texture_assets: Res<TextureAssets>,
) {
    commands.spawn((
        Name::new("Bird"),
        Bird,
        Gravity,
        MovementController::default(),
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(random::<f32>(), random::<f32>(), random::<f32>()),
                ..default()
            },
            texture: texture_assets.bird.clone(),
            transform: Transform::from_translation(config.translation).with_scale(Vec3::splat(0.1)),
            ..default()
        },
        Velocity(config.velocity),
    ));
}

fn setup(mut commands: Commands) {
    commands.queue(SpawnBird {
        translation: Vec3::ZERO,
        velocity: Vec2::ZERO,
    });

    commands.spawn((
        Camera2dBundle::default(),
        MovementController {
            intent: Vec2::new(1.0, 0.0),
            horizontal_speed: CAMERA_SPEED,
            // We never need to move the camera vertically.
            vertical_speed: 0.0,
        },
        Velocity(Vec2::new(CAMERA_SPEED, 0.0)),
    ));
}

fn input(input: Res<ButtonInput<KeyCode>>, mut moveable: Query<&mut MovementController>) {
    // Per the genre, flappy games characters continually move at a constant rate along the x axis.
    // So our "intent" is always positive for `x`.
    let mut intent = Vec2::new(1.0, 0.0);

    if input.just_pressed(KeyCode::Space) {
        // We'd like all the birds to "flap".
        intent.y = 1.0;
    }
    for mut controller in &mut moveable {
        controller.intent = intent;
    }
}

fn reproduction(
    mut commands: Commands,
    birds: Query<(&Transform, &Velocity), With<Bird>>,
    flock_settings: Res<FlockSettings>,
    time: Res<Time>,
) {
    let bird_count = birds.iter().count();
    if bird_count < flock_settings.max_birds {
        for (transform, velocity) in &birds {
            if random::<f32>() < flock_settings.reproduction_chance * time.delta_seconds() {
                commands.queue(SpawnBird {
                    translation: transform.translation,
                    velocity: velocity.0,
                });
            }
        }
    }
}

// Physics Plugin
//
// We handle all physics systems on the `FixedUpdate` schedule. This helps avoid bugs that can crop
// up when using `Update` due to it not being guaranteed to run at fixed intervals.
fn physics_plugin(app: &mut App) {
    app.init_resource::<PhysicsSettings>();
    app.add_systems(
        FixedUpdate,
        (check_for_collisions, apply_movement, drift, velocity)
            .chain()
            .in_set(AppSet::Physics),
    );
}

#[derive(Clone, Component, Debug, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Gravity;

#[derive(Resource)]
struct PhysicsSettings {
    pub gravity: f32,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self { gravity: 250.0 }
    }
}

#[derive(Component)]
struct MovementController {
    // The direction the attached entity intends to move.
    pub intent: Vec2,

    // Maximum speed in world units per second.
    // 1 world unit = 1 pixel when using the default 2D camera and no physics
    // engine.
    // TODO: is the above still true?
    pub horizontal_speed: f32,
    pub vertical_speed: f32,
}

impl Default for MovementController {
    fn default() -> Self {
        Self {
            intent: Vec2::ZERO,
            // Per the genre, our heroes move at the speed of the camera.
            horizontal_speed: CAMERA_SPEED,
            // Set to higher than gravity to avoid plummeting to the ground!
            vertical_speed: 300.0,
        }
    }
}

fn check_for_collisions(
    mut commands: Commands,
    birds: Query<(Entity, &Transform), With<Bird>>,
    flock_settings: Res<FlockSettings>,
    obstacles: Query<&Transform, With<Obstacle>>,
) {
    for (bird, bird_transform) in &birds {
        let bounding_circle = BoundingCircle::new(
            bird_transform.translation.truncate(),
            flock_settings.bird_size / 2.,
        );

        for obstacle in &obstacles {
            let bounding_box = Aabb2d::new(
                obstacle.translation.truncate(),
                obstacle.scale.truncate() / 2.0,
            );

            if bounding_circle.intersects(&bounding_box) {
                commands.entity(bird).despawn_recursive();
            }
        }
    }
}

fn drift(
    mut birds: Query<(&mut MovementController, &Transform), With<Bird>>,
    camera: Query<&Transform, With<Camera>>,
    flock_settings: Res<FlockSettings>,
) {
    let Ok(camera_transform) = camera.get_single() else {
        return;
    };

    for (mut controller, bird_transform) in &mut birds {
        let x_distance = camera_transform.translation.x - bird_transform.translation.x;
        if x_distance < 100.0 {
            // Brownian drift
            controller.horizontal_speed += (random::<f32>() - 0.5) * flock_settings.drift.x;
            controller.vertical_speed += (random::<f32>() - 0.5) * flock_settings.drift.y;
        } else {
            // Gradually move back toward horizontal center of screen.
            controller.horizontal_speed -= (bird_transform.translation.x
                - camera_transform.translation.x)
                * flock_settings.drift.x
                / 10.0;
        }
    }
}

fn velocity(mut q: Query<(&Velocity, &mut Transform)>, time: Res<Time>) {
    for (v, mut t) in q.iter_mut() {
        t.translation += v.0.extend(0.0) * time.delta_seconds();
    }
}

fn apply_movement(
    physics_settings: Res<PhysicsSettings>,
    mut moveable: Query<(&MovementController, &mut Velocity, Option<&Gravity>)>,
    time: Res<Time>,
) {
    for (controller, mut velocity, gravity) in &mut moveable {
        // We know we always want horizontal speed to be constant, so we assign rather than
        // increment here. Individual variations are created by slightly varying the entity's
        // `horizontal_speed`.
        velocity.0.x = controller.intent.x * controller.horizontal_speed;

        // Note that we don't involve delta time here, because we wish to increment the velocity
        // by a known amount after a keypress. The time between frames is immaterial.
        velocity.0.y += controller.intent.y * controller.vertical_speed;
        if gravity.is_some() {
            // By contrast, we DO want delta time here, because gravity should be applied
            // evenly no matter what the FPS.
            velocity.0.y -= physics_settings.gravity * time.delta_seconds();
        }

        // To avoid players spamming the "flap" key and obtaining higher and higher vertical
        // speeds, we use `clamp` to guarantee a maximum amount of "lift". Note that this also has
        // the helpful effect of setting "terminal velocity". In other words, if our gravity is
        // 100.0, we can never fall faster than -100.0.
        velocity.0.y = velocity
            .0
            .y
            .clamp(-physics_settings.gravity, controller.vertical_speed);
    }
}

// Terrain Plugin
fn terrain_plugin(app: &mut App) {
    app.init_resource::<TerrainSettings>();
    app.add_systems(
        Update,
        (generate_terrain, terrain_cleanup).in_set(AppSet::Playing),
    );
}

#[derive(Resource)]
struct TerrainSettings {
    pub chunk_size: f32,
    pub cleanup_distance: f32,
    // TODO: what is this value, actually?
    pub game_height: f32,
    pub gap_variability: f32,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            chunk_size: 300.0,
            cleanup_distance: 1500.0,
            game_height: 500.0,
            gap_variability: 0.9,
        }
    }
}

struct SpawnTerrain {
    index: i32,
}

impl Command for SpawnTerrain {
    fn apply(self, world: &mut World) {
        world.run_system_once_with(self, spawn_terrain);
    }
}

#[derive(Component)]
struct Obstacle;

fn generate_terrain(
    camera: Query<&Transform, With<Camera>>,
    mut commands: Commands,
    mut last_chunk: Local<i32>,
    terrain_settings: Res<TerrainSettings>,
) {
    if let Ok(transform) = camera.get_single() {
        let chunk = (transform.translation.x / terrain_settings.chunk_size).floor() as i32;
        if chunk > 0 && chunk != *last_chunk {
            // The camera has crossed a chunk boundary. Generate a new chunk offscreen to the
            // right.
            commands.queue(SpawnTerrain { index: chunk + 3 });

            *last_chunk = chunk;
        }
    }
}

fn spawn_terrain(
    In(config): In<SpawnTerrain>,
    mut commands: Commands,
    terrain_settings: Res<TerrainSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    // TODO: mostly yoinked from the original, could still use a bit of clarification
    let Ok(primary_window) = window.get_single() else {
        return;
    };
    let x_pos = terrain_settings.chunk_size * config.index as f32;
    // generate some terrain within x_pos..x_pos+width
    let gap_y_pos =
        terrain_settings.game_height * (random::<f32>() - 0.5) * terrain_settings.gap_variability;
    // TODO: magic numbers
    let pillar_width = 50.0 + 110.0 * random::<f32>();
    // make the gap no narrower than the pillar is wide
    let gap_size = (65.0 + 250.0 * random::<f32>()).max(pillar_width);
    for (top_y_pos, bottom_y_pos) in [
        (-primary_window.height() * 0.5, gap_y_pos - gap_size * 0.5),
        (gap_y_pos + gap_size * 0.5, primary_window.height() * 0.5),
    ] {
        let pillar_origin = Vec2::new(x_pos, (top_y_pos + bottom_y_pos) * 0.5);
        let pillar_size = Vec2::new(pillar_width, bottom_y_pos - top_y_pos);
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.25, 0.25, 0.75),
                    ..default()
                },
                transform: Transform::from_translation(pillar_origin.extend(0.0))
                    .with_scale(pillar_size.extend(1.0)),
                ..default()
            })
            .insert(Obstacle);
    }
}

fn terrain_cleanup(
    mut commands: Commands,
    obstacles: Query<(Entity, &Transform), With<Obstacle>>,
    camera: Query<&Transform, With<Camera>>,
    terrain_settings: Res<TerrainSettings>,
) {
    let Ok(camera_transform) = camera.get_single() else {
        return;
    };
    for (obstacle, obstacle_transform) in &obstacles {
        // Remove obstacles at the left
        if obstacle_transform.translation.x
            < camera_transform.translation.x - terrain_settings.cleanup_distance
        {
            commands.entity(obstacle).despawn();
        }
    }
}
