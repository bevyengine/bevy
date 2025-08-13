//! This example provides a 2D benchmark.
//!
//! Usage: spawn more entities by clicking on the screen.

use core::time::Duration;
use std::str::FromStr;

use argh::FromArgs;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::basic::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::AlphaMode2d,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use rand::{seq::IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const BIRDS_PER_SECOND: u32 = 10000;
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: f32 = 750.;
const BIRD_SCALE: f32 = 0.15;
const BIRD_TEXTURE_SIZE: usize = 256;
const HALF_BIRD_SIZE: f32 = BIRD_TEXTURE_SIZE as f32 * BIRD_SCALE * 0.5;

#[derive(Resource)]
struct BevyCounter {
    pub count: usize,
    pub color: Color,
}

#[derive(Component)]
struct Bird {
    velocity: Vec3,
}

#[derive(FromArgs, Resource)]
/// `bevymark` sprite / 2D mesh stress test
struct Args {
    /// whether to use sprite or mesh2d
    #[argh(option, default = "Mode::Sprite")]
    mode: Mode,

    /// whether to step animations by a fixed amount such that each frame is the same across runs.
    /// If spawning waves, all are spawned up-front to immediately start rendering at the heaviest
    /// load.
    #[argh(switch)]
    benchmark: bool,

    /// how many birds to spawn per wave.
    #[argh(option, default = "0")]
    per_wave: usize,

    /// the number of waves to spawn.
    #[argh(option, default = "0")]
    waves: usize,

    /// whether to vary the material data in each instance.
    #[argh(switch)]
    vary_per_instance: bool,

    /// the number of different textures from which to randomly select the material color. 0 means no textures.
    #[argh(option, default = "1")]
    material_texture_count: usize,

    /// generate z values in increasing order rather than randomly
    #[argh(switch)]
    ordered_z: bool,

    /// the alpha mode used to spawn the sprites
    #[argh(option, default = "AlphaMode::Blend")]
    alpha_mode: AlphaMode,
}

#[derive(Default, Clone)]
enum Mode {
    #[default]
    Sprite,
    Mesh2d,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sprite" => Ok(Self::Sprite),
            "mesh2d" => Ok(Self::Mesh2d),
            _ => Err(format!(
                "Unknown mode: '{s}', valid modes: 'sprite', 'mesh2d'"
            )),
        }
    }
}

#[derive(Default, Clone)]
enum AlphaMode {
    Opaque,
    #[default]
    Blend,
    AlphaMask,
}

impl FromStr for AlphaMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "opaque" => Ok(Self::Opaque),
            "blend" => Ok(Self::Blend),
            "alpha_mask" => Ok(Self::AlphaMask),
            _ => Err(format!(
                "Unknown alpha mode: '{s}', valid modes: 'opaque', 'blend', 'alpha_mask'"
            )),
        }
    }
}

const FIXED_TIMESTEP: f32 = 0.2;

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "BevyMark".into(),
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(args)
        .insert_resource(BevyCounter {
            count: 0,
            color: Color::WHITE,
        })
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, scheduled_spawner)
        .add_systems(
            Update,
            (
                mouse_handler,
                movement_system,
                collision_system,
                counter_system,
            ),
        )
        .insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
            FIXED_TIMESTEP,
        )))
        .run();
}

#[derive(Resource)]
struct BirdScheduled {
    waves: usize,
    per_wave: usize,
}

fn scheduled_spawner(
    mut commands: Commands,
    args: Res<Args>,
    window: Single<&Window>,
    mut scheduled: ResMut<BirdScheduled>,
    mut counter: ResMut<BevyCounter>,
    bird_resources: ResMut<BirdResources>,
) {
    if scheduled.waves > 0 {
        let bird_resources = bird_resources.into_inner();
        spawn_birds(
            &mut commands,
            args.into_inner(),
            &window.resolution,
            &mut counter,
            scheduled.per_wave,
            bird_resources,
            None,
            scheduled.waves - 1,
        );

        scheduled.waves -= 1;
    }
}

#[derive(Resource)]
struct BirdResources {
    textures: Vec<Handle<Image>>,
    materials: Vec<Handle<ColorMaterial>>,
    quad: Handle<Mesh>,
    color_rng: ChaCha8Rng,
    material_rng: ChaCha8Rng,
    velocity_rng: ChaCha8Rng,
    transform_rng: ChaCha8Rng,
}

#[derive(Component)]
struct StatsText;

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<ColorMaterial>>,
    images: ResMut<Assets<Image>>,
    window: Single<&Window>,
    counter: ResMut<BevyCounter>,
) {
    warn!(include_str!("warning_string.txt"));

    let args = args.into_inner();
    let images = images.into_inner();

    let mut textures = Vec::with_capacity(args.material_texture_count.max(1));
    if matches!(args.mode, Mode::Sprite) || args.material_texture_count > 0 {
        textures.push(asset_server.load("branding/icon.png"));
    }
    init_textures(&mut textures, args, images);

    let material_assets = material_assets.into_inner();
    let materials = init_materials(args, &textures, material_assets);

    let mut bird_resources = BirdResources {
        textures,
        materials,
        quad: meshes.add(Rectangle::from_size(Vec2::splat(BIRD_TEXTURE_SIZE as f32))),
        // We're seeding the PRNG here to make this example deterministic for testing purposes.
        // This isn't strictly required in practical use unless you need your app to be deterministic.
        color_rng: ChaCha8Rng::seed_from_u64(42),
        material_rng: ChaCha8Rng::seed_from_u64(42),
        velocity_rng: ChaCha8Rng::seed_from_u64(42),
        transform_rng: ChaCha8Rng::seed_from_u64(42),
    };

    let font = TextFont {
        font_size: 40.0,
        ..Default::default()
    };

    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.75)),
            GlobalZIndex(i32::MAX),
        ))
        .with_children(|p| {
            p.spawn((Text::default(), StatsText)).with_children(|p| {
                p.spawn((
                    TextSpan::new("Bird Count: "),
                    font.clone(),
                    TextColor(LIME.into()),
                ));
                p.spawn((TextSpan::new(""), font.clone(), TextColor(AQUA.into())));
                p.spawn((
                    TextSpan::new("\nFPS (raw): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ));
                p.spawn((TextSpan::new(""), font.clone(), TextColor(AQUA.into())));
                p.spawn((
                    TextSpan::new("\nFPS (SMA): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ));
                p.spawn((TextSpan::new(""), font.clone(), TextColor(AQUA.into())));
                p.spawn((
                    TextSpan::new("\nFPS (EMA): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ));
                p.spawn((TextSpan::new(""), font.clone(), TextColor(AQUA.into())));
            });
        });

    let mut scheduled = BirdScheduled {
        per_wave: args.per_wave,
        waves: args.waves,
    };

    if args.benchmark {
        let counter = counter.into_inner();
        for wave in (0..scheduled.waves).rev() {
            spawn_birds(
                &mut commands,
                args,
                &window.resolution,
                counter,
                scheduled.per_wave,
                &mut bird_resources,
                Some(wave),
                wave,
            );
        }
        scheduled.waves = 0;
    }
    commands.insert_resource(bird_resources);
    commands.insert_resource(scheduled);
}

fn mouse_handler(
    mut commands: Commands,
    args: Res<Args>,
    time: Res<Time>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    bird_resources: ResMut<BirdResources>,
    mut counter: ResMut<BevyCounter>,
    mut rng: Local<Option<ChaCha8Rng>>,
    mut wave: Local<usize>,
) {
    let Ok(window) = window.single() else {
        return;
    };

    if rng.is_none() {
        // We're seeding the PRNG here to make this example deterministic for testing purposes.
        // This isn't strictly required in practical use unless you need your app to be deterministic.
        *rng = Some(ChaCha8Rng::seed_from_u64(42));
    }
    let rng = rng.as_mut().unwrap();

    if mouse_button_input.just_released(MouseButton::Left) {
        counter.color = Color::linear_rgb(rng.random(), rng.random(), rng.random());
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (BIRDS_PER_SECOND as f64 * time.delta_secs_f64()) as usize;
        spawn_birds(
            &mut commands,
            args.into_inner(),
            &window.resolution,
            &mut counter,
            spawn_count,
            bird_resources.into_inner(),
            None,
            *wave,
        );
        *wave += 1;
    }
}

fn bird_velocity_transform(
    half_extents: Vec2,
    mut translation: Vec3,
    velocity_rng: &mut ChaCha8Rng,
    waves: Option<usize>,
    dt: f32,
) -> (Transform, Vec3) {
    let mut velocity = Vec3::new(MAX_VELOCITY * (velocity_rng.random::<f32>() - 0.5), 0., 0.);

    if let Some(waves) = waves {
        // Step the movement and handle collisions as if the wave had been spawned at fixed time intervals
        // and with dt-spaced frames of simulation
        for _ in 0..(waves * (FIXED_TIMESTEP / dt).round() as usize) {
            step_movement(&mut translation, &mut velocity, dt);
            handle_collision(half_extents, &translation, &mut velocity);
        }
    }
    (
        Transform::from_translation(translation).with_scale(Vec3::splat(BIRD_SCALE)),
        velocity,
    )
}

const FIXED_DELTA_TIME: f32 = 1.0 / 60.0;

fn spawn_birds(
    commands: &mut Commands,
    args: &Args,
    primary_window_resolution: &WindowResolution,
    counter: &mut BevyCounter,
    spawn_count: usize,
    bird_resources: &mut BirdResources,
    waves_to_simulate: Option<usize>,
    wave: usize,
) {
    let bird_x = (primary_window_resolution.width() / -2.) + HALF_BIRD_SIZE;
    let bird_y = (primary_window_resolution.height() / 2.) - HALF_BIRD_SIZE;

    let half_extents = 0.5 * primary_window_resolution.size();

    let color = counter.color;
    let current_count = counter.count;

    match args.mode {
        Mode::Sprite => {
            let batch = (0..spawn_count)
                .map(|count| {
                    let bird_z = if args.ordered_z {
                        (current_count + count) as f32 * 0.00001
                    } else {
                        bird_resources.transform_rng.random::<f32>()
                    };

                    let (transform, velocity) = bird_velocity_transform(
                        half_extents,
                        Vec3::new(bird_x, bird_y, bird_z),
                        &mut bird_resources.velocity_rng,
                        waves_to_simulate,
                        FIXED_DELTA_TIME,
                    );

                    let color = if args.vary_per_instance {
                        Color::linear_rgb(
                            bird_resources.color_rng.random(),
                            bird_resources.color_rng.random(),
                            bird_resources.color_rng.random(),
                        )
                    } else {
                        color
                    };
                    (
                        Sprite {
                            image: bird_resources
                                .textures
                                .choose(&mut bird_resources.material_rng)
                                .unwrap()
                                .clone(),
                            color,
                            ..default()
                        },
                        transform,
                        Bird { velocity },
                    )
                })
                .collect::<Vec<_>>();
            commands.spawn_batch(batch);
        }
        Mode::Mesh2d => {
            let batch = (0..spawn_count)
                .map(|count| {
                    let bird_z = if args.ordered_z {
                        (current_count + count) as f32 * 0.00001
                    } else {
                        bird_resources.transform_rng.random::<f32>()
                    };

                    let (transform, velocity) = bird_velocity_transform(
                        half_extents,
                        Vec3::new(bird_x, bird_y, bird_z),
                        &mut bird_resources.velocity_rng,
                        waves_to_simulate,
                        FIXED_DELTA_TIME,
                    );

                    let material =
                        if args.vary_per_instance || args.material_texture_count > args.waves {
                            bird_resources
                                .materials
                                .choose(&mut bird_resources.material_rng)
                                .unwrap()
                                .clone()
                        } else {
                            bird_resources.materials[wave % bird_resources.materials.len()].clone()
                        };
                    (
                        Mesh2d(bird_resources.quad.clone()),
                        MeshMaterial2d(material),
                        transform,
                        Bird { velocity },
                    )
                })
                .collect::<Vec<_>>();
            commands.spawn_batch(batch);
        }
    }

    counter.count += spawn_count;
    counter.color = Color::linear_rgb(
        bird_resources.color_rng.random(),
        bird_resources.color_rng.random(),
        bird_resources.color_rng.random(),
    );
}

fn step_movement(translation: &mut Vec3, velocity: &mut Vec3, dt: f32) {
    translation.x += velocity.x * dt;
    translation.y += velocity.y * dt;
    velocity.y += GRAVITY * dt;
}

fn movement_system(
    args: Res<Args>,
    time: Res<Time>,
    mut bird_query: Query<(&mut Bird, &mut Transform)>,
) {
    let dt = if args.benchmark {
        FIXED_DELTA_TIME
    } else {
        time.delta_secs()
    };
    for (mut bird, mut transform) in &mut bird_query {
        step_movement(&mut transform.translation, &mut bird.velocity, dt);
    }
}

fn handle_collision(half_extents: Vec2, translation: &Vec3, velocity: &mut Vec3) {
    if (velocity.x > 0. && translation.x + HALF_BIRD_SIZE > half_extents.x)
        || (velocity.x <= 0. && translation.x - HALF_BIRD_SIZE < -half_extents.x)
    {
        velocity.x = -velocity.x;
    }
    let velocity_y = velocity.y;
    if velocity_y < 0. && translation.y - HALF_BIRD_SIZE < -half_extents.y {
        velocity.y = -velocity_y;
    }
    if translation.y + HALF_BIRD_SIZE > half_extents.y && velocity_y > 0.0 {
        velocity.y = 0.0;
    }
}
fn collision_system(window: Query<&Window>, mut bird_query: Query<(&mut Bird, &Transform)>) {
    let Ok(window) = window.single() else {
        return;
    };

    let half_extents = 0.5 * window.size();

    for (mut bird, transform) in &mut bird_query {
        handle_collision(half_extents, &transform.translation, &mut bird.velocity);
    }
}

fn counter_system(
    diagnostics: Res<DiagnosticsStore>,
    counter: Res<BevyCounter>,
    query: Single<Entity, With<StatsText>>,
    mut writer: TextUiWriter,
) {
    let text = *query;

    if counter.is_changed() {
        *writer.text(text, 2) = counter.count.to_string();
    }

    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(raw) = fps.value() {
            *writer.text(text, 4) = format!("{raw:.2}");
        }
        if let Some(sma) = fps.average() {
            *writer.text(text, 6) = format!("{sma:.2}");
        }
        if let Some(ema) = fps.smoothed() {
            *writer.text(text, 8) = format!("{ema:.2}");
        }
    };
}

fn init_textures(textures: &mut Vec<Handle<Image>>, args: &Args, images: &mut Assets<Image>) {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut color_rng = ChaCha8Rng::seed_from_u64(42);
    while textures.len() < args.material_texture_count {
        let pixel = [
            color_rng.random(),
            color_rng.random(),
            color_rng.random(),
            255,
        ];
        textures.push(images.add(Image::new_fill(
            Extent3d {
                width: BIRD_TEXTURE_SIZE as u32,
                height: BIRD_TEXTURE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &pixel,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )));
    }
}

fn init_materials(
    args: &Args,
    textures: &[Handle<Image>],
    assets: &mut Assets<ColorMaterial>,
) -> Vec<Handle<ColorMaterial>> {
    let capacity = if args.vary_per_instance {
        args.per_wave * args.waves
    } else {
        args.material_texture_count.max(args.waves)
    }
    .max(1);

    let alpha_mode = match args.alpha_mode {
        AlphaMode::Opaque => AlphaMode2d::Opaque,
        AlphaMode::Blend => AlphaMode2d::Blend,
        AlphaMode::AlphaMask => AlphaMode2d::Mask(0.5),
    };

    let mut materials = Vec::with_capacity(capacity);
    materials.push(assets.add(ColorMaterial {
        color: Color::WHITE,
        texture: textures.first().cloned(),
        alpha_mode,
        ..default()
    }));

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut color_rng = ChaCha8Rng::seed_from_u64(42);
    let mut texture_rng = ChaCha8Rng::seed_from_u64(42);
    materials.extend(
        std::iter::repeat_with(|| {
            assets.add(ColorMaterial {
                color: Color::srgb_u8(color_rng.random(), color_rng.random(), color_rng.random()),
                texture: textures.choose(&mut texture_rng).cloned(),
                alpha_mode,
                ..default()
            })
        })
        .take(capacity - materials.len()),
    );

    materials
}
