//! Demonstrates realtime dynamic raytraced lighting using Bevy Solari.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use argh::FromArgs;
use bevy::{
    camera::CameraMainTextureUsages,
    gltf::GltfMaterialName,
    prelude::*,
    render::render_resource::TextureUsages,
    scene::SceneInstanceReady,
    solari::{
        pathtracer::{Pathtracer, PathtracingPlugin},
        prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
    },
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy::anti_alias::dlss::{
    Dlss, DlssProjectId, DlssRayReconstructionFeature, DlssRayReconstructionSupported,
};

/// `bevy_solari` demo.
#[derive(FromArgs, Resource, Clone, Copy)]
struct Args {
    /// use the reference pathtracer instead of the realtime lighting system.
    #[argh(switch)]
    pathtracer: Option<bool>,
}

fn main() {
    let args: Args = argh::from_env();

    let mut app = App::new();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy_asset::uuid::uuid!(
        "5417916c-0291-4e3f-8f65-326c1858ab96" // Don't copy paste this - generate your own UUID!
    )));

    app.add_plugins((DefaultPlugins, SolariPlugins, CameraControllerPlugin))
        .insert_resource(args)
        .add_systems(Startup, setup);

    if args.pathtracer == Some(true) {
        app.add_plugins(PathtracingPlugin);
    } else {
        app.add_systems(Update, (pause_scene, toggle_lights, patrol_path));
        app.add_systems(PostUpdate, update_text);
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    args: Res<Args>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
) {
    commands
        .spawn((
            SceneRoot(
                asset_server.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset("https://github.com/bevyengine/bevy_asset_files/raw/2a5950295a8b6d9d051d59c0df69e87abcda58c3/pica_pica/mini_diorama_01.glb")
                ),
            ),
            Transform::from_scale(Vec3::splat(10.0)),
        ))
        .observe(add_raytracing_meshes_on_scene_load);

    commands
        .spawn((
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("https://github.com/bevyengine/bevy_asset_files/raw/2a5950295a8b6d9d051d59c0df69e87abcda58c3/pica_pica/robot_01.glb")
            )),
            Transform::from_scale(Vec3::splat(2.0))
                .with_translation(Vec3::new(-2.0, 0.05, -2.1))
                .with_rotation(Quat::from_rotation_y(PI / 2.0)),
            PatrolPath {
                path: vec![
                    (Vec3::new(-2.0, 0.05, -2.1), Quat::from_rotation_y(PI / 2.0)),
                    (Vec3::new(2.2, 0.05, -2.1), Quat::from_rotation_y(0.0)),
                    (
                        Vec3::new(2.2, 0.05, 2.1),
                        Quat::from_rotation_y(3.0 * PI / 2.0),
                    ),
                    (Vec3::new(-2.0, 0.05, 2.1), Quat::from_rotation_y(PI)),
                ],
                i: 0,
            },
        ))
        .observe(add_raytracing_meshes_on_scene_load);

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: false, // Solari replaces shadow mapping
            ..default()
        },
        Transform::from_rotation(Quat::from_xyzw(
            -0.13334629,
            -0.86597735,
            -0.3586996,
            0.3219264,
        )),
    ));

    let mut camera = commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        CameraController {
            walk_speed: 3.0,
            run_speed: 10.0,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(0.219417, 2.5764852, 6.9718704)).with_rotation(
            Quat::from_xyzw(-0.1466768, 0.013738206, 0.002037309, 0.989087),
        ),
        // Msaa::Off and CameraMainTextureUsages with STORAGE_BINDING are required for Solari
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Msaa::Off,
    ));

    if args.pathtracer == Some(true) {
        camera.insert(Pathtracer::default());
    } else {
        camera.insert(SolariLighting::default());
    }

    // Using DLSS Ray Reconstruction for denoising (and cheaper rendering via upscaling) is _highly_ recommended when using Solari
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() {
        camera.insert(Dlss::<DlssRayReconstructionFeature> {
            perf_quality_mode: Default::default(),
            reset: Default::default(),
            _phantom_data: Default::default(),
        });
    }

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn add_raytracing_meshes_on_scene_load(
    scene_ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    mesh_query: Query<(
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GltfMaterialName>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    args: Res<Args>,
) {
    for descendant in children.iter_descendants(scene_ready.entity) {
        if let Ok((Mesh3d(mesh_handle), MeshMaterial3d(material_handle), material_name)) =
            mesh_query.get(descendant)
        {
            // Add raytracing mesh component
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh_handle.clone()));

            // Ensure meshes are Solari compatible
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
                let vertex_count = mesh.count_vertices();
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; vertex_count]);
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_TANGENT,
                    vec![[0.0, 0.0, 0.0, 0.0]; vertex_count],
                );
            }
            if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
                mesh.generate_tangents().unwrap();
            }
            if mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
                mesh.remove_attribute(Mesh::ATTRIBUTE_UV_1);
            }

            // Prevent rasterization if using pathtracer
            if args.pathtracer == Some(true) {
                commands.entity(descendant).remove::<Mesh3d>();
            }

            // Adjust scene materials to better demo Solari features
            if material_name.map(|s| s.0.as_str()) == Some("material") {
                let material = materials.get_mut(material_handle).unwrap();
                material.emissive = LinearRgba::BLACK;
            }
            if material_name.map(|s| s.0.as_str()) == Some("Lights") {
                let material = materials.get_mut(material_handle).unwrap();
                material.emissive =
                    LinearRgba::from(Color::srgb(0.941, 0.714, 0.043)) * 1_000_000.0;
                material.alpha_mode = AlphaMode::Opaque;
                material.specular_transmission = 0.0;

                commands.insert_resource(RobotLightMaterial(material_handle.clone()));
            }
            if material_name.map(|s| s.0.as_str()) == Some("Glass_Dark_01") {
                let material = materials.get_mut(material_handle).unwrap();
                material.alpha_mode = AlphaMode::Opaque;
                material.specular_transmission = 0.0;
            }
        }
    }
}

fn pause_scene(mut time: ResMut<Time<Virtual>>, key_input: Res<ButtonInput<KeyCode>>) {
    if key_input.just_pressed(KeyCode::Space) {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    }
}

#[derive(Resource)]
struct RobotLightMaterial(Handle<StandardMaterial>);

fn toggle_lights(
    key_input: Res<ButtonInput<KeyCode>>,
    robot_light_material: Option<Res<RobotLightMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    directional_light: Query<Entity, With<DirectionalLight>>,
    mut commands: Commands,
) {
    if key_input.just_pressed(KeyCode::Digit1) {
        if let Ok(directional_light) = directional_light.single() {
            commands.entity(directional_light).despawn();
        } else {
            commands.spawn((
                DirectionalLight {
                    illuminance: light_consts::lux::FULL_DAYLIGHT,
                    shadows_enabled: false, // Solari replaces shadow mapping
                    ..default()
                },
                Transform::from_rotation(Quat::from_xyzw(
                    -0.13334629,
                    -0.86597735,
                    -0.3586996,
                    0.3219264,
                )),
            ));
        }
    }

    if key_input.just_pressed(KeyCode::Digit2)
        && let Some(robot_light_material) = robot_light_material
    {
        let material = materials.get_mut(&robot_light_material.0).unwrap();
        if material.emissive == LinearRgba::BLACK {
            material.emissive = LinearRgba::from(Color::srgb(0.941, 0.714, 0.043)) * 1_000_000.0;
        } else {
            material.emissive = LinearRgba::BLACK;
        }
    }
}

#[derive(Component)]
struct PatrolPath {
    path: Vec<(Vec3, Quat)>,
    i: usize,
}

fn patrol_path(mut query: Query<(&mut PatrolPath, &mut Transform)>, time: Res<Time<Virtual>>) {
    for (mut path, mut transform) in query.iter_mut() {
        let (mut target_position, mut target_rotation) = path.path[path.i];
        let mut distance_to_target = transform.translation.distance(target_position);
        if distance_to_target < 0.01 {
            transform.translation = target_position;
            transform.rotation = target_rotation;

            path.i = (path.i + 1) % path.path.len();
            (target_position, target_rotation) = path.path[path.i];
            distance_to_target = transform.translation.distance(target_position);
        }

        let direction = (target_position - transform.translation).normalize();
        let movement = direction * time.delta_secs();

        if movement.length() > distance_to_target {
            transform.translation = target_position;
            transform.rotation = target_rotation;
        } else {
            transform.translation += movement;
        }
    }
}

fn update_text(
    mut text: Single<&mut Text>,
    robot_light_material: Option<Res<RobotLightMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    directional_light: Query<Entity, With<DirectionalLight>>,
    time: Res<Time<Virtual>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
) {
    text.0.clear();

    if time.is_paused() {
        text.0.push_str("(Space): Resume");
    } else {
        text.0.push_str("(Space): Pause");
    }

    if directional_light.single().is_ok() {
        text.0.push_str("\n(1): Disable directional light");
    } else {
        text.0.push_str("\n(1): Enable directional light");
    }

    match robot_light_material.and_then(|m| materials.get(&m.0)) {
        Some(robot_light_material) if robot_light_material.emissive != LinearRgba::BLACK => {
            text.0.push_str("\n(2): Disable robot emissive light");
        }
        _ => {
            text.0.push_str("\n(2): Enable robot emissive light");
        }
    }

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() {
        text.0
            .push_str("\nDenoising: DLSS Ray Reconstruction enabled");
    } else {
        text.0
            .push_str("\nDenoising: DLSS Ray Reconstruction not supported");
    }

    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    text.0
        .push_str("\nDenoising: App not compiled with DLSS support");
}
