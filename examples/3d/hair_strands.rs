//! Renders strand-based hair as view-facing ribbons with an anisotropic hair material.
//!
//! Two heads are grown a procedural hairstyle each. The strands live in a
//! [`HairStrands`] asset; the ribbon mesh is regenerated automatically whenever
//! that asset changes, which is what pressing `Space` does, and the culling
//! bounds follow the strand width set on the material. Zoom out to watch the
//! level of detail thin the strands while the hair keeps its coverage.

use std::f32::consts::{PI, TAU};

use bevy::{
    hair_strands::prelude::*,
    light::{CascadeShadowConfigBuilder, NotShadowCaster},
    math::ops,
    prelude::*,
};
use chacha20::ChaCha8Rng;
use rand::{RngExt, SeedableRng};

const INSTRUCTIONS: &str = "\
Controls
--------
Space: regrow the hair
Up / Down: thicker / thinner strands
Left / Right: shift the primary highlight
R: toggle camera rotation
Z / X: camera nearer / further (level of detail)";

const HEAD_RADIUS: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
            ..default()
        })
        .insert_resource(CameraOrbit {
            angle: 0.0,
            distance: 3.6,
            rotating: true,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (orbit_camera, orbit_point_light, handle_input))
        .run();
}

/// The camera's orbit about the heads: where it is on the circle, how far out
/// (`Z` / `X`), and whether it is moving (`R`).
#[derive(Resource)]
struct CameraOrbit {
    angle: f32,
    distance: f32,
    rotating: bool,
}

/// The seed a head's hairstyle was grown from, so it can be regrown differently.
#[derive(Component)]
struct Hairstyle {
    seed: u64,
    style: Style,
}

/// The overall shape of a hairstyle.
#[derive(Clone, Copy)]
enum Style {
    /// Long, mostly straight strands that hang under gravity.
    Straight,
    /// Shorter strands with a pronounced wave.
    Wavy,
}

/// The point light that circles the heads to show the highlights move.
#[derive(Component)]
struct OrbitingLight;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut hair_materials: ResMut<Assets<HairMaterial>>,
    mut hair_strands: ResMut<Assets<HairStrands>>,
) {
    // Ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.32, 0.36),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, -1.2, 0.0),
    ));

    let head_mesh = meshes.add(Sphere::new(HEAD_RADIUS).mesh().ico(6).unwrap());
    let skin = standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.62, 0.5),
        perceptual_roughness: 0.7,
        ..default()
    });

    let heads = [
        (
            Vec3::new(-0.9, 0.0, 0.0),
            Style::Straight,
            HairMaterial {
                base_color: Color::srgb(0.16, 0.08, 0.04),
                specular_color: Color::srgb(0.55, 0.5, 0.45),
                secondary_specular_color: Color::srgb(0.5, 0.28, 0.12),
                root_width: 0.012,
                tip_width: 0.003,
                ..default()
            },
        ),
        (
            Vec3::new(0.9, 0.0, 0.0),
            Style::Wavy,
            HairMaterial {
                base_color: Color::srgb(0.72, 0.5, 0.2),
                specular_color: Color::srgb(1.0, 0.95, 0.85),
                secondary_specular_color: Color::srgb(0.9, 0.65, 0.3),
                root_width: 0.014,
                tip_width: 0.004,
                specular_exponent: 200.0,
                secondary_exponent: 30.0,
                root_occlusion: 0.45,
                ..default()
            },
        ),
    ];

    for (index, (position, style, material)) in heads.into_iter().enumerate() {
        let seed = 1000 + index as u64;
        commands
            .spawn((
                Mesh3d(head_mesh.clone()),
                MeshMaterial3d(skin.clone()),
                Transform::from_translation(position),
            ))
            .with_child((
                HairStrands3d(hair_strands.add(grow_hair(seed, style))),
                MeshMaterial3d(hair_materials.add(material)),
                Hairstyle { seed, style },
            ));
    }

    // Sun, with shadows so the hair shades the heads and the ground.
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.8, -0.9)),
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 6.0,
            maximum_distance: 20.0,
            ..default()
        }
        .build(),
    ));

    // A warm point light that circles the heads.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.85, 0.7),
            intensity: 120_000.0,
            range: 8.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 1.5, 2.0),
        OrbitingLight,
        children![(
            Mesh3d(meshes.add(Sphere::new(0.05))),
            MeshMaterial3d(standard_materials.add(StandardMaterial {
                emissive: LinearRgba::new(4.0, 3.0, 2.0, 1.0),
                ..default()
            })),
            NotShadowCaster,
        )],
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.8, 3.6).looking_at(Vec3::new(0.0, 0.1, 0.0), Vec3::Y),
    ));

    commands.spawn((
        Text::new(INSTRUCTIONS),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Grows a procedural hairstyle rooted on the upper part of a unit head.
///
/// Each strand starts on the sphere surface pointing along the surface normal
/// and is then bent, segment by segment, by gravity and a per-strand wave, while
/// being pushed back outside the head whenever it would pass through it.
fn grow_hair(seed: u64, style: Style) -> HairStrands {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let (strand_count, segment_count, length_range, wave_strength, gravity_strength) = match style {
        Style::Straight => (2_400, 14, 0.7..1.1, 0.03, 0.45),
        Style::Wavy => (2_000, 16, 0.45..0.75, 0.16, 0.3),
    };

    let mut strands = HairStrands::new();
    for _ in 0..strand_count {
        // Roots are spread over the top of the head, thinning out toward the
        // hairline. `y` is biased upwards so the crown is densest.
        let y = 1.0 - ops::powf(rng.random::<f32>(), 1.6) * 0.75;
        let ring = (1.0 - y * y).sqrt();
        let angle = rng.random_range(0.0..TAU);
        let normal = Vec3::new(ring * ops::cos(angle), y, ring * ops::sin(angle));

        let length = rng.random_range(length_range.clone());
        let segment = length / segment_count as f32;
        let wave_phase = rng.random_range(0.0..TAU);
        let wave_frequency = rng.random_range(4.0..7.0);
        // A sideways axis for the wave, perpendicular to the strand's start.
        let wave_axis = normal.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);

        let mut point = normal * (HEAD_RADIUS + 0.005);
        let mut direction = normal;
        let mut points = Vec::with_capacity(segment_count + 1);
        points.push(point);

        for i in 1..=segment_count {
            let t = i as f32 / segment_count as f32;
            // Gravity wins gradually along the strand; the wave is strongest
            // partway down so the roots stay flat against the head.
            let wave = ops::sin(t * wave_frequency + wave_phase) * wave_strength * t.sqrt();
            let pull = Vec3::NEG_Y * (gravity_strength * (0.4 + t)) + wave_axis * wave;
            direction = (direction + pull).normalize();
            point += direction * segment;

            // Keep the strand out of the head.
            let distance = point.length();
            let min_distance = HEAD_RADIUS + 0.01;
            if distance < min_distance {
                point = point / distance * min_distance;
                direction = (direction - normal * direction.dot(normal) * 0.5).normalize();
            }
            points.push(point);
        }
        strands.push(HairStrand::new(points));
    }
    strands
}

fn orbit_camera(
    mut orbit: ResMut<CameraOrbit>,
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    if orbit.rotating {
        orbit.angle += time.delta_secs() * 0.25;
    }
    let (angle, distance) = (orbit.angle, orbit.distance);
    for mut transform in &mut camera {
        transform.translation = Vec3::new(
            ops::sin(angle) * distance,
            0.8 * distance / 3.6,
            ops::cos(angle) * distance,
        );
        transform.look_at(Vec3::new(0.0, 0.1, 0.0), Vec3::Y);
    }
}

fn orbit_point_light(time: Res<Time>, mut light: Query<&mut Transform, With<OrbitingLight>>) {
    let angle = time.elapsed_secs() * 0.9 + PI;
    for mut transform in &mut light {
        transform.translation = Vec3::new(ops::sin(angle) * 2.2, 1.4, ops::cos(angle) * 2.2);
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut orbit: ResMut<CameraOrbit>,
    mut hair_strands: ResMut<Assets<HairStrands>>,
    mut hair_materials: ResMut<Assets<HairMaterial>>,
    mut heads: Query<(
        &HairStrands3d,
        &MeshMaterial3d<HairMaterial>,
        &mut Hairstyle,
    )>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        orbit.rotating = !orbit.rotating;
    }
    // Nearer or further, in the same proportion each second, so the level of
    // detail can be watched thinning the strands as they shrink on screen.
    let zoom = match (keys.pressed(KeyCode::KeyZ), keys.pressed(KeyCode::KeyX)) {
        (true, false) => 0.5,
        (false, true) => 2.0,
        _ => 1.0,
    };
    if zoom != 1.0 {
        orbit.distance = (orbit.distance * ops::powf(zoom, time.delta_secs())).clamp(1.5, 60.0);
    }

    let regrow = keys.just_pressed(KeyCode::Space);
    let width_delta = match (
        keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::ArrowDown),
    ) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    } * time.delta_secs()
        * 0.01;
    let shift_delta = match (
        keys.pressed(KeyCode::ArrowRight),
        keys.pressed(KeyCode::ArrowLeft),
    ) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    } * time.delta_secs()
        * 0.3;

    for (strands, material, mut hairstyle) in &mut heads {
        if regrow && let Some(mut asset) = hair_strands.get_mut(&strands.0) {
            hairstyle.seed = hairstyle
                .seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            // Replacing the asset is enough: the ribbon mesh is rebuilt for us.
            *asset = grow_hair(hairstyle.seed, hairstyle.style);
        }

        if (width_delta != 0.0 || shift_delta != 0.0)
            && let Some(mut material) = hair_materials.get_mut(&material.0)
        {
            material.root_width = (material.root_width + width_delta).clamp(0.002, 0.05);
            material.tip_width = (material.tip_width + width_delta * 0.3).clamp(0.001, 0.02);
            material.specular_shift = (material.specular_shift + shift_delta).clamp(-0.6, 0.6);
        }
    }
}
