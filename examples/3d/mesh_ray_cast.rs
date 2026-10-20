//! Demonstrates how to use the [`MeshRayCast`] system parameter to chain multiple ray casts
//! and bounce off of surfaces.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes::css, core_pipeline::tonemapping::Tonemapping, math::vec3,
    picking::backend::ray::RayMap, post_process::bloom::Bloom, prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, bouncing_raycast)
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

const MAX_BOUNCES: usize = 64;
const LASER_SPEED: f32 = 0.03;

fn bouncing_raycast(
    mut ray_cast: MeshRayCast,
    mut gizmos: Gizmos,
    time: Res<Time>,
    // The ray map stores rays cast by the cursor
    ray_map: Res<RayMap>,
) {
    // Cast an automatically moving ray and bounce it off of surfaces
    let t = ops::cos((time.elapsed_secs() - 4.0).max(0.0) * LASER_SPEED) * PI;
    let ray_pos = Vec3::new(ops::sin(t), ops::cos(3.0 * t) * 0.5, ops::cos(t)) * 0.5;
    let ray_dir = Dir3::new(-ray_pos).unwrap();
    let ray = Ray3d::new(ray_pos, ray_dir);
    gizmos.sphere(ray_pos, 0.1, Color::WHITE);
    bounce_ray(ray, &mut ray_cast, &mut gizmos, Color::from(css::RED));

    // Cast a ray from the cursor and bounce it off of surfaces
    for (_, ray) in ray_map.iter() {
        bounce_ray(*ray, &mut ray_cast, &mut gizmos, Color::from(css::GREEN));
    }
}

// Bounces a ray off of surfaces `MAX_BOUNCES` times.
fn bounce_ray(mut ray: Ray3d, ray_cast: &mut MeshRayCast, gizmos: &mut Gizmos, color: Color) {
    let mut intersections = Vec::with_capacity(MAX_BOUNCES + 1);
    intersections.push((ray.origin, Color::srgb(30.0, 0.0, 0.0)));

    for i in 0..MAX_BOUNCES {
        // Cast the ray and get the first hit
        let Some((_, hit)) = ray_cast
            .cast_ray(ray, &MeshRayCastSettings::default())
            .first()
        else {
            break;
        };

        // Draw the point of intersection and add it to the list
        let brightness = 1.0 + 10.0 * (1.0 - i as f32 / MAX_BOUNCES as f32);
        intersections.push((hit.point, Color::BLACK.mix(&color, brightness)));
        gizmos.sphere(hit.point, 0.005, Color::BLACK.mix(&color, brightness * 2.0));

        // Reflect the ray off of the surface
        ray.direction = Dir3::new(ray.direction.reflect(hit.normal)).unwrap();
        ray.origin = hit.point + ray.direction * 1e-6;
    }
    gizmos.linestrip_gradient(intersections);
}

// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Make a box of planes facing inward so the laser gets trapped inside
    let plane_mesh = meshes.add(Plane3d::default());
    let plane_material = materials.add(Color::from(css::GRAY).with_alpha(0.01));
    let create_plane = move |translation, rotation| {
        (
            Transform::from_translation(translation)
                .with_rotation(Quat::from_scaled_axis(rotation)),
            Mesh3d(plane_mesh.clone()),
            MeshMaterial3d(plane_material.clone()),
        )
    };

    commands.spawn(create_plane(vec3(0.0, 0.5, 0.0), Vec3::X * PI));
    commands.spawn(create_plane(vec3(0.0, -0.5, 0.0), Vec3::ZERO));
    commands.spawn(create_plane(vec3(0.5, 0.0, 0.0), Vec3::Z * FRAC_PI_2));
    commands.spawn(create_plane(vec3(-0.5, 0.0, 0.0), Vec3::Z * -FRAC_PI_2));
    commands.spawn(create_plane(vec3(0.0, 0.0, 0.5), Vec3::X * -FRAC_PI_2));
    commands.spawn(create_plane(vec3(0.0, 0.0, -0.5), Vec3::X * FRAC_PI_2));

    // Light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.1, 0.2, 0.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.5, 1.5, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
        Tonemapping::TonyMcMapface,
        Bloom::default(),
    ));
}
