//! This example showcases pbr atmospheric scattering

use std::f32::consts::PI;

use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    input::mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    pbr::{light_consts::lux, Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder},
    prelude::*,
    render::camera::Exposure,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .add_systems(
            Update,
            (
                dynamic_scene,
                pan_camera,
                smooth_camera_movement.after(pan_camera),
            ),
        )
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    let initial_distance = 1.0;
    let initial_transform =
        Transform::from_xyz(-initial_distance, 0.1, 0.0).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3d::default(),
        // HDR is required for atmospheric scattering to be properly applied to the scene
        Camera {
            hdr: true,
            ..default()
        },
        initial_transform.clone(),
        CameraOrbit {
            target_transform: initial_transform,
            distance: initial_distance,
        },
        // This is the component that enables atmospheric scattering for a camera
        Atmosphere::EARTH,
        // The scene is in units of 10km, so we need to scale up the
        // aerial view lut distance and set the scene scale accordingly.
        // Most usages of this feature will not need to adjust this.
        AtmosphereSettings {
            aerial_view_lut_max_distance: 3.2e5,
            scene_units_to_m: 1e+4,
            ..Default::default()
        },
        // The directional light illuminance  used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure { ev100: 12.0 },
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        Tonemapping::AcesFitted,
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
    ));

    let sun_transform = Transform::from_xyz(1.0, 1.0, -0.3).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            // lux::RAW_SUNLIGHT is recommended for use with this feature, since
            // other values approximate sunlight *post-scattering* in various
            // conditions. RAW_SUNLIGHT in comparison is the illuminance of the
            // sun unfiltered by the atmosphere, so it is the proper input for
            // sunlight to be filtered by the atmosphere.
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        sun_transform.clone(),
        SunOrbit {
            target_transform: sun_transform,
        },
    ));
}

#[derive(Component)]
struct Terrain;

#[derive(Component)]
struct CameraOrbit {
    target_transform: Transform,
    distance: f32,
}

#[derive(Component)]
struct SunOrbit {
    target_transform: Transform,
}

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

    // light probe spheres
    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, -0.1).with_scale(Vec3::splat(0.05)),
    ));

    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, 0.1).with_scale(Vec3::splat(0.05)),
    ));

    let tall_cylinder_mesh = meshes.add(Cylinder {
        radius: 0.5,
        half_height: 5.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(tall_cylinder_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        })),
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));

    // Terrain
    commands.spawn((
        Terrain,
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/terrain/terrain.glb")),
        ),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));
}

fn dynamic_scene(mut suns: Query<&mut Transform, With<DirectionalLight>>, time: Res<Time>) {
    // suns.iter_mut()
    //     .for_each(|mut tf| tf.rotate_x(-time.delta_secs() * PI / 10.0));
}

fn pan_camera(
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut camera_query: Query<(&Transform, &mut CameraOrbit), With<Camera3d>>,
    mut sun_query: Query<(&Transform, &mut SunOrbit), (With<DirectionalLight>, Without<Camera3d>)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_query_view: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
) {
    let (camera_transform, mut camera_orbit) = camera_query.single_mut();
    let (sun_transform, mut sun_orbit) = sun_query.single_mut();
    let (camera, camera_global_transform) = camera_query_view.single();
    let window = windows.single();

    // Handle zoom with mouse wheel
    for ev in scroll_evr.read() {
        let zoom_factor = camera_orbit.distance * 0.1; // Scale zoom speed with distance
        camera_orbit.distance = (camera_orbit.distance - ev.y * zoom_factor).clamp(0.5, 20.0);

        // Update target transform to maintain direction but change distance
        let direction = camera_orbit.target_transform.translation.normalize();
        camera_orbit.target_transform.translation = direction * camera_orbit.distance;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    for ev in motion_evr.read() {
        if mouse_button.pressed(MouseButton::Left) {
            let orbit_speed = 0.005;

            // Calculate rotations
            let yaw_rotation = Quat::from_axis_angle(Vec3::Y, -ev.delta.x * orbit_speed);
            let pitch_rotation =
                Quat::from_axis_angle(camera_transform.local_x().into(), -ev.delta.y * orbit_speed);

            // Get current position and apply rotations
            let current_pos = camera_orbit.target_transform.translation;
            let rotated_pos = yaw_rotation * pitch_rotation * current_pos;

            // Update target transform
            camera_orbit.target_transform.translation = rotated_pos;
            camera_orbit.target_transform.look_at(Vec3::ZERO, Vec3::Y);
        } else if mouse_button.pressed(MouseButton::Right) {
            let Ok(ray) = camera.viewport_to_world(camera_global_transform, cursor_pos) else {
                continue;
            };

            let sphere_radius = 10.0;

            if let Some(intersection) = ray_sphere_intersection(
                ray.origin,
                Vec3::splat(1.0) * Vec3::from(ray.direction),
                Vec3::ZERO,
                sphere_radius,
            ) {
                let mut target = sun_orbit.target_transform;
                target.translation = intersection;
                target.look_at(Vec3::ZERO, Vec3::Y);
                sun_orbit.target_transform = target;
            }
        }
    }
}

fn smooth_camera_movement(
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &CameraOrbit), With<Camera3d>>,
    mut sun_query: Query<(&mut Transform, &SunOrbit), (With<DirectionalLight>, Without<Camera3d>)>,
) {
    let damping = 1.0 - (-8.0 * time.delta_secs()).exp();

    // Update camera
    if let Ok((mut transform, orbit)) = camera_query.get_single_mut() {
        transform.translation = transform
            .translation
            .lerp(orbit.target_transform.translation, damping);
        transform.rotation = transform
            .rotation
            .slerp(orbit.target_transform.rotation, damping);
    }

    // Update sun
    if let Ok((mut transform, orbit)) = sun_query.get_single_mut() {
        transform.translation = transform
            .translation
            .lerp(orbit.target_transform.translation, damping);
        transform.rotation = transform
            .rotation
            .slerp(orbit.target_transform.rotation, damping);
    }
}

// Helper function to calculate ray-sphere intersection
fn ray_sphere_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
) -> Option<Vec3> {
    let oc = ray_origin - sphere_center;
    let a = ray_direction.dot(ray_direction);
    let b = 2.0 * oc.dot(ray_direction);
    let c = oc.dot(oc) - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        None
    } else {
        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        Some(ray_origin + ray_direction * t)
    }
}
