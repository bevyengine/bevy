//! Demonstrates the `specular_anti_aliasing` toggle on [`StandardMaterial`].
//!
//! A normal-mapped cube rotates under an orbiting point light. Press Space
//! to toggle specular anti-aliasing; roughness, variance, threshold and
//! light intensity can also be adjusted live with the keyboard.

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    asset::RenderAssetUsages,
    camera::Hdr,
    color::palettes::css::WHITE,
    core_pipeline::{
        prepass::{DepthPrepass, MotionVectorPrepass},
        tonemapping::Tonemapping::AcesFitted,
    },
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    input::mouse::{MouseMotion, MouseWheel},
    light::Skybox,
    math::{ops, vec3, Affine2},
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        camera::{MipBias, TemporalJitter},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

type TaaComponents = (
    TemporalAntiAliasing,
    TemporalJitter,
    MipBias,
    DepthPrepass,
    MotionVectorPrepass,
);

const CAMERA_MIN_DISTANCE: f32 = 1.5;
const CAMERA_MAX_DISTANCE: f32 = 12.0;
const CAMERA_INITIAL_DISTANCE: f32 = 5.0;

#[derive(Resource)]
struct Settings {
    aa: bool,
    taa: bool,
    roughness: f32,
    sigma: f32,
    kappa: f32,
    light_intensity: f32,
    paused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            aa: true,
            taa: false,
            roughness: 0.06,
            sigma: 0.5,
            kappa: 0.18,
            light_intensity: 1_000_000.0,
            paused: false,
        }
    }
}

#[derive(Component)]
struct Subject;

#[derive(Component)]
struct OrbitLight {
    angle: f32,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
        .init_resource::<Settings>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                apply_settings,
                apply_taa,
                spin,
                orbit_light,
                drag_rotate,
                wheel_zoom,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
) {
    let cube = meshes.add(
        Mesh::from(Cuboid::new(1.5, 1.5, 1.5))
            .with_generated_tangents()
            .expect("Failed to generate tangents"),
    );

    let normal_map = images.add(build_voronoi_normal_map(512, 72));

    commands.spawn((
        Mesh3d(cube),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.57, 0.6),
            normal_map_texture: Some(normal_map),
            metallic: 1.0,
            reflectance: 1.0,
            uv_transform: Affine2::from_scale(Vec2::splat(2.5)),
            perceptual_roughness: settings.roughness,
            specular_anti_aliasing: settings.aa,
            specular_anti_aliasing_screen_space_variance: settings.sigma,
            specular_anti_aliasing_threshold: settings.kappa,
            ..default()
        })),
        Transform::default(),
        Subject,
    ));

    commands.spawn((
        PointLight {
            color: WHITE.into(),
            intensity: settings.light_intensity,
            range: 50.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 2.5),
        OrbitLight { angle: 0.0 },
    ));

    let env_specular: Handle<Image> =
        asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");
    let env_diffuse: Handle<Image> =
        asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2");

    commands.spawn((
        Camera3d::default(),
        Hdr,
        Msaa::Sample4,
        Transform::from_xyz(0.0, 0.5, CAMERA_INITIAL_DISTANCE).looking_at(Vec3::ZERO, Vec3::Y),
        AcesFitted,
        Bloom::NATURAL,
        EnvironmentMapLight {
            diffuse_map: env_diffuse,
            specular_map: env_specular.clone(),
            intensity: 400.0,
            ..default()
        },
        Skybox {
            image: Some(env_specular),
            brightness: 150.0,
            ..default()
        },
    ));

    commands.spawn((
        help_text(&settings),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10),
            left: px(10),
            ..default()
        },
    ));
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut settings: ResMut<Settings>,
) {
    let dt = time.delta_secs();

    if keyboard.just_pressed(KeyCode::Space) {
        settings.aa = !settings.aa;
    }
    if keyboard.just_pressed(KeyCode::KeyT) {
        settings.taa = !settings.taa;
    }
    if keyboard.just_pressed(KeyCode::KeyG) {
        settings.paused = !settings.paused;
    }

    if keyboard.pressed(KeyCode::KeyQ) {
        settings.roughness = (settings.roughness + 0.25 * dt).min(1.0);
    }
    if keyboard.pressed(KeyCode::KeyA) {
        settings.roughness = (settings.roughness - 0.25 * dt).max(0.0);
    }
    if keyboard.pressed(KeyCode::KeyW) {
        settings.sigma = (settings.sigma + 0.6 * dt).min(3.0);
    }
    if keyboard.pressed(KeyCode::KeyS) {
        settings.sigma = (settings.sigma - 0.6 * dt).max(0.0);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        settings.kappa = (settings.kappa + 0.2 * dt).min(1.0);
    }
    if keyboard.pressed(KeyCode::KeyD) {
        settings.kappa = (settings.kappa - 0.2 * dt).max(0.0);
    }
    if keyboard.pressed(KeyCode::KeyR) {
        settings.light_intensity = (settings.light_intensity * (1.0 + dt)).min(50_000_000.0);
    }
    if keyboard.pressed(KeyCode::KeyF) {
        settings.light_intensity = (settings.light_intensity * (1.0 - dt)).max(10_000.0);
    }
}

fn apply_settings(
    settings: Res<Settings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    subject: Query<&MeshMaterial3d<StandardMaterial>, With<Subject>>,
    mut lights: Query<&mut PointLight, With<OrbitLight>>,
    mut text_query: Query<&mut Text>,
) {
    if !settings.is_changed() {
        return;
    }
    for handle in subject.iter() {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.specular_anti_aliasing = settings.aa;
            material.perceptual_roughness = settings.roughness;
            material.specular_anti_aliasing_screen_space_variance = settings.sigma;
            material.specular_anti_aliasing_threshold = settings.kappa;
        }
    }
    for mut light in lights.iter_mut() {
        light.intensity = settings.light_intensity;
    }
    for mut text in text_query.iter_mut() {
        *text = help_text(&settings);
    }
}

fn apply_taa(
    settings: Res<Settings>,
    mut last: Local<Option<bool>>,
    mut commands: Commands,
    camera: Single<(Entity, &mut Msaa), With<Camera3d>>,
) {
    if *last == Some(settings.taa) {
        return;
    }
    *last = Some(settings.taa);

    let (entity, mut msaa) = camera.into_inner();
    if settings.taa {
        *msaa = Msaa::Off;
        commands
            .entity(entity)
            .insert(TemporalAntiAliasing::default());
    } else {
        commands.entity(entity).remove::<TaaComponents>();
        *msaa = Msaa::Sample4;
    }
}

fn spin(
    mut q: Query<&mut Transform, With<Subject>>,
    time: Res<Time>,
    settings: Res<Settings>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if settings.paused || mouse.pressed(MouseButton::Left) {
        return;
    }
    let dt = time.delta_secs();
    for mut tr in q.iter_mut() {
        tr.rotate_local_y(0.3 * dt);
        tr.rotate_local_x(0.18 * dt);
    }
}

fn orbit_light(
    mut q: Query<(&mut Transform, &mut OrbitLight)>,
    time: Res<Time>,
    settings: Res<Settings>,
) {
    if settings.paused {
        return;
    }
    let dt = time.delta_secs();
    for (mut tr, mut orbit) in q.iter_mut() {
        orbit.angle += 0.9 * dt;
        tr.translation = vec3(
            ops::sin(orbit.angle) * 3.0,
            2.0,
            ops::cos(orbit.angle) * 3.0,
        );
    }
}

fn drag_rotate(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut q: Query<&mut Transform, With<Subject>>,
) {
    if !mouse.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }
    let mut total = Vec2::ZERO;
    for ev in motion.read() {
        total += ev.delta;
    }
    if total == Vec2::ZERO {
        return;
    }
    let yaw = -total.x * 0.005;
    let pitch = -total.y * 0.005;
    for mut tr in q.iter_mut() {
        let rot_y = Quat::from_rotation_y(yaw);
        let rot_x = Quat::from_rotation_x(pitch);
        tr.rotation = rot_y * rot_x * tr.rotation;
    }
}

fn wheel_zoom(
    mut scroll: MessageReader<MouseWheel>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let mut delta = 0.0;
    for ev in scroll.read() {
        delta += ev.y;
    }
    if delta == 0.0 {
        return;
    }
    let dir = camera.translation.normalize_or_zero();
    if dir == Vec3::ZERO {
        return;
    }
    let dist = camera.translation.length();
    let new_dist = (dist - delta * 0.4).clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
    camera.translation = dir * new_dist;
    camera.look_at(Vec3::ZERO, Vec3::Y);
}

fn build_voronoi_normal_map(size: u32, cell_count: u32) -> Image {
    const GROOVE_WIDTH: f32 = 0.012;
    const GROOVE_DEPTH: f32 = 1.4;
    const NORMAL_STRENGTH: f32 = 7.0;

    let mut rng: u32 = 0x9E37_79B9;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng
    };

    let points: Vec<(f32, f32)> = (0..cell_count)
        .map(|_| {
            let x = (next() & 0xffff) as f32 / 65535.0;
            let y = (next() & 0xffff) as f32 / 65535.0;
            (x, y)
        })
        .collect();

    let height = |fx: f32, fy: f32| -> f32 {
        let mut d1 = f32::INFINITY;
        let mut d2 = f32::INFINITY;
        for &(px, py) in &points {
            for oy in -1..=1 {
                for ox in -1..=1 {
                    let dx = fx - (px + ox as f32);
                    let dy = fy - (py + oy as f32);
                    let d = ops::hypot(dx, dy);
                    if d < d1 {
                        d2 = d1;
                        d1 = d;
                    } else if d < d2 {
                        d2 = d;
                    }
                }
            }
        }

        let border_dist = d2 - d1;
        let t = (border_dist / GROOVE_WIDTH).min(1.0);
        let smoothed = t * t * (3.0 - 2.0 * t);
        -GROOVE_DEPTH * (1.0 - smoothed)
    };

    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let inv = 1.0 / size as f32;

    for y in 0..size {
        for x in 0..size {
            let fx = (x as f32 + 0.5) * inv;
            let fy = (y as f32 + 0.5) * inv;

            let dh_dx = (height(fx + inv, fy) - height(fx - inv, fy)) * 0.5 / inv;
            let dh_dy = (height(fx, fy + inv) - height(fx, fy - inv)) * 0.5 / inv;

            let nx = -dh_dx * NORMAL_STRENGTH;
            let ny = -dh_dy * NORMAL_STRENGTH;
            let len = (nx * nx + ny * ny + 1.0).sqrt();
            let nx = nx / len;
            let ny = ny / len;
            let nz = 1.0 / len;

            data.push(((nx * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
            data.push(((ny * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
            data.push(((nz * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
            data.push(255);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    image
}

fn help_text(settings: &Settings) -> Text {
    let aa = if settings.aa { "ON" } else { "OFF" };
    let taa = if settings.taa { "ON" } else { "OFF" };
    let paused = if settings.paused { "paused" } else { "playing" };
    Text::new(format!(
        "Specular AA: {aa}    TAA: {taa}    animation: {paused}\n\
         roughness    (Q/A): {:.3}\n\
         variance     (W/S): {:.3}\n\
         threshold    (E/D): {:.3}\n\
         light        (R/F): {:.0}\n\
         \n\
         Space: toggle specular AA    T: toggle TAA    G: pause",
        settings.roughness, settings.sigma, settings.kappa, settings.light_intensity,
    ))
}
