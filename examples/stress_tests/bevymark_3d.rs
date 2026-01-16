//! This example provides a 3D benchmark.
//!
//! Usage: spawn more entities by clicking with the left mouse button.

use core::time::Duration;
use std::str::FromStr;

use argh::FromArgs;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::basic::*,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use bevy_asset::RenderAssetTransferPriority;
use rand::{seq::IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const CUBES_PER_SECOND: u32 = 10000;
const GRAVITY: f32 = -9.8;
const MAX_VELOCITY: f32 = 10.;
const CUBE_SCALE: f32 = 1.0;
const CUBE_TEXTURE_SIZE: usize = 256;
const HALF_CUBE_SIZE: f32 = CUBE_SCALE * 0.5;
const VOLUME_WIDTH: usize = 50;
const VOLUME_SIZE: Vec3 = Vec3::splat(VOLUME_WIDTH as f32);

#[derive(Resource)]
struct BevyCounter {
    pub count: usize,
    pub color: Color,
}

#[derive(Component)]
struct Cube {
    velocity: Vec3,
}

#[derive(FromArgs, Resource)]
/// `bevymark_3d` cube stress test
struct Args {
    /// whether to step animations by a fixed amount such that each frame is the same across runs.
    /// If spawning waves, all are spawned up-front to immediately start rendering at the heaviest
    /// load.
    #[argh(switch)]
    benchmark: bool,

    /// how many cubes to spawn per wave.
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

    /// the alpha mode used to spawn the cubes
    #[argh(option, default = "AlphaMode::Opaque")]
    alpha_mode: AlphaMode,
}

#[derive(Default, Clone)]
enum AlphaMode {
    #[default]
    Opaque,
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
                    title: "BevyMark 3D".into(),
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
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
struct CubeScheduled {
    waves: usize,
    per_wave: usize,
}

fn scheduled_spawner(
    mut commands: Commands,
    args: Res<Args>,
    mut scheduled: ResMut<CubeScheduled>,
    mut counter: ResMut<BevyCounter>,
    cube_resources: ResMut<CubeResources>,
) {
    if scheduled.waves > 0 {
        let cube_resources = cube_resources.into_inner();
        spawn_cubes(
            &mut commands,
            args.into_inner(),
            &mut counter,
            scheduled.per_wave,
            cube_resources,
            None,
            scheduled.waves - 1,
        );

        scheduled.waves -= 1;
    }
}

#[derive(Resource)]
struct CubeResources {
    _textures: Vec<Handle<Image>>,
    materials: Vec<Handle<StandardMaterial>>,
    cube_mesh: Handle<Mesh>,
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
    material_assets: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
    counter: ResMut<BevyCounter>,
) {
    let args = args.into_inner();
    let images = images.into_inner();

    let mut textures = Vec::with_capacity(args.material_texture_count.max(1));
    if args.material_texture_count > 0 {
        textures.push(asset_server.load("branding/icon.png"));
    }
    init_textures(&mut textures, args, images);

    let material_assets = material_assets.into_inner();
    let materials = init_materials(args, &textures, material_assets);

    let mut cube_resources = CubeResources {
        _textures: textures,
        materials,
        cube_mesh: meshes.add(Cuboid::from_size(Vec3::splat(CUBE_SCALE))),
        color_rng: ChaCha8Rng::seed_from_u64(42),
        material_rng: ChaCha8Rng::seed_from_u64(12),
        velocity_rng: ChaCha8Rng::seed_from_u64(97),
        transform_rng: ChaCha8Rng::seed_from_u64(26),
    };

    let font = TextFont {
        font_size: 40.0,
        ..Default::default()
    };

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(VOLUME_SIZE * 1.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(5)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.75)),
        GlobalZIndex(i32::MAX),
        children![(
            Text::default(),
            StatsText,
            children![
                (
                    TextSpan::new("Cube Count: "),
                    font.clone(),
                    TextColor(LIME.into()),
                ),
                (TextSpan::new(""), font.clone(), TextColor(AQUA.into())),
                (
                    TextSpan::new("\nFPS (raw): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ),
                (TextSpan::new(""), font.clone(), TextColor(AQUA.into())),
                (
                    TextSpan::new("\nFPS (SMA): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ),
                (TextSpan::new(""), font.clone(), TextColor(AQUA.into())),
                (
                    TextSpan::new("\nFPS (EMA): "),
                    font.clone(),
                    TextColor(LIME.into()),
                ),
                (TextSpan::new(""), font.clone(), TextColor(AQUA.into()))
            ]
        )],
    ));

    let mut scheduled = CubeScheduled {
        per_wave: args.per_wave,
        waves: args.waves,
    };

    if args.benchmark {
        let counter = counter.into_inner();
        for wave in (0..scheduled.waves).rev() {
            spawn_cubes(
                &mut commands,
                args,
                counter,
                scheduled.per_wave,
                &mut cube_resources,
                Some(wave),
                wave,
            );
        }
        scheduled.waves = 0;
    }
    commands.insert_resource(cube_resources);
    commands.insert_resource(scheduled);
}

fn mouse_handler(
    mut commands: Commands,
    args: Res<Args>,
    time: Res<Time>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    cube_resources: ResMut<CubeResources>,
    mut counter: ResMut<BevyCounter>,
    mut rng: Local<Option<ChaCha8Rng>>,
    mut wave: Local<usize>,
) {
    if rng.is_none() {
        *rng = Some(ChaCha8Rng::seed_from_u64(42));
    }
    let rng = rng.as_mut().unwrap();

    if mouse_button_input.just_released(MouseButton::Left) {
        counter.color = Color::linear_rgb(rng.random(), rng.random(), rng.random());
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (CUBES_PER_SECOND as f64 * time.delta_secs_f64()) as usize;
        spawn_cubes(
            &mut commands,
            args.into_inner(),
            &mut counter,
            spawn_count,
            cube_resources.into_inner(),
            None,
            *wave,
        );
        *wave += 1;
    }
}

fn cube_velocity_transform(
    mut translation: Vec3,
    velocity_rng: &mut ChaCha8Rng,
    waves: Option<usize>,
    dt: f32,
) -> (Transform, Vec3) {
    let mut velocity = Vec3::new(0., 0., MAX_VELOCITY * velocity_rng.random::<f32>());

    if let Some(waves) = waves {
        for _ in 0..(waves * (FIXED_TIMESTEP / dt).round() as usize) {
            step_movement(&mut translation, &mut velocity, dt);
            handle_collision(&translation, &mut velocity);
        }
    }
    (Transform::from_translation(translation), velocity)
}

const FIXED_DELTA_TIME: f32 = 1.0 / 60.0;

fn spawn_cubes(
    commands: &mut Commands,
    args: &Args,
    counter: &mut BevyCounter,
    spawn_count: usize,
    cube_resources: &mut CubeResources,
    waves_to_simulate: Option<usize>,
    wave: usize,
) {
    let batch_material = cube_resources.materials[wave % cube_resources.materials.len()].clone();

    let spawn_y = VOLUME_SIZE.y / 2.0 - HALF_CUBE_SIZE;
    let spawn_z = -VOLUME_SIZE.z / 2.0 + HALF_CUBE_SIZE;

    let batch = (0..spawn_count)
        .map(|_| {
            let spawn_pos = Vec3::new(
                (cube_resources.transform_rng.random::<f32>() - 0.5) * VOLUME_SIZE.x,
                spawn_y,
                spawn_z,
            );

            let (transform, velocity) = cube_velocity_transform(
                spawn_pos,
                &mut cube_resources.velocity_rng,
                waves_to_simulate,
                FIXED_DELTA_TIME,
            );

            let material = if args.vary_per_instance {
                cube_resources
                    .materials
                    .choose(&mut cube_resources.material_rng)
                    .unwrap()
                    .clone()
            } else {
                batch_material.clone()
            };

            (
                Mesh3d(cube_resources.cube_mesh.clone()),
                MeshMaterial3d(material),
                transform,
                Cube { velocity },
            )
        })
        .collect::<Vec<_>>();
    commands.spawn_batch(batch);

    counter.count += spawn_count;
    counter.color = Color::linear_rgb(
        cube_resources.color_rng.random(),
        cube_resources.color_rng.random(),
        cube_resources.color_rng.random(),
    );
}

fn step_movement(translation: &mut Vec3, velocity: &mut Vec3, dt: f32) {
    translation.x += velocity.x * dt;
    translation.y += velocity.y * dt;
    translation.z += velocity.z * dt;
    velocity.y += GRAVITY * dt;
}

fn movement_system(
    args: Res<Args>,
    time: Res<Time>,
    mut cube_query: Query<(&mut Cube, &mut Transform)>,
) {
    let dt = if args.benchmark {
        FIXED_DELTA_TIME
    } else {
        time.delta_secs()
    };
    for (mut cube, mut transform) in &mut cube_query {
        step_movement(&mut transform.translation, &mut cube.velocity, dt);
    }
}

fn handle_collision(translation: &Vec3, velocity: &mut Vec3) {
    if (velocity.x > 0. && translation.x + HALF_CUBE_SIZE > VOLUME_SIZE.x / 2.0)
        || (velocity.x <= 0. && translation.x - HALF_CUBE_SIZE < -VOLUME_SIZE.x / 2.0)
    {
        velocity.x = -velocity.x;
    }
    if (velocity.z > 0. && translation.z + HALF_CUBE_SIZE > VOLUME_SIZE.z / 2.0)
        || (velocity.z <= 0. && translation.z - HALF_CUBE_SIZE < -VOLUME_SIZE.z / 2.0)
    {
        velocity.z = -velocity.z;
    }

    let velocity_y = velocity.y;
    if velocity_y < 0. && translation.y - HALF_CUBE_SIZE < -VOLUME_SIZE.y / 2.0 {
        velocity.y = -velocity_y;
    }
    if translation.y + HALF_CUBE_SIZE > VOLUME_SIZE.y / 2.0 && velocity_y > 0.0 {
        velocity.y = 0.0;
    }
}

fn collision_system(mut cube_query: Query<(&mut Cube, &Transform)>) {
    cube_query.par_iter_mut().for_each(|(mut cube, transform)| {
        handle_collision(&transform.translation, &mut cube.velocity);
    });
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
                width: CUBE_TEXTURE_SIZE as u32,
                height: CUBE_TEXTURE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &pixel,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
            RenderAssetTransferPriority::default(),
        )));
    }
}

fn init_materials(
    args: &Args,
    textures: &[Handle<Image>],
    assets: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let mut capacity = if args.vary_per_instance {
        args.per_wave * args.waves
    } else {
        args.material_texture_count.max(args.waves)
    };
    if !args.benchmark {
        capacity = capacity.max(256);
    }
    capacity = capacity.max(1);

    let alpha_mode = match args.alpha_mode {
        AlphaMode::Opaque => bevy::prelude::AlphaMode::Opaque,
        AlphaMode::Blend => bevy::prelude::AlphaMode::Blend,
        AlphaMode::AlphaMask => bevy::prelude::AlphaMode::Mask(0.5),
    };

    let mut materials = Vec::with_capacity(capacity);
    materials.push(assets.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: textures.first().cloned(),
        alpha_mode,
        ..default()
    }));

    let mut color_rng = ChaCha8Rng::seed_from_u64(42);
    let mut texture_rng = ChaCha8Rng::seed_from_u64(42);
    materials.extend(
        std::iter::repeat_with(|| {
            assets.add(StandardMaterial {
                base_color: Color::linear_rgb(
                    color_rng.random(),
                    color_rng.random(),
                    color_rng.random(),
                ),
                base_color_texture: textures.choose(&mut texture_rng).cloned(),
                alpha_mode,
                ..default()
            })
        })
        .take(capacity - materials.len()),
    );

    materials
}
