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
use bevy::anti_aliasing::dlss::{
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
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_directional_light);

    if args.pathtracer == Some(true) {
        app.add_plugins(PathtracingPlugin);
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
                        .from_asset("models/PicaPica/pica_pica_-_mini_diorama_01.glb"),
                ),
            ),
            Transform::from_scale(Vec3::splat(10.0)),
        ))
        .observe(add_raytracing_meshes_on_scene_load);

    // TODO: Animate robot
    commands
        .spawn((
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/PicaPica/pica_pica_-_robot_01.glb"),
            )),
            Transform::from_scale(Vec3::splat(3.0)).with_translation(Vec3::new(0.0, 0.05, 0.0)),
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
}

fn add_raytracing_meshes_on_scene_load(
    trigger: On<SceneInstanceReady>,
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
    for descendant in children.iter_descendants(trigger.target()) {
        if let Ok((Mesh3d(mesh_handle), MeshMaterial3d(material_handle), material_name)) =
            mesh_query.get(descendant)
        {
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

            // Add raytracing mesh component
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh_handle.clone()));

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
                material.emissive = LinearRgba::from(Color::srgb(0.941, 0.714, 0.043)) * 300_000.0;
                material.alpha_mode = AlphaMode::Opaque;
                material.specular_transmission = 0.0;
            }
            if material_name.map(|s| s.0.as_str()) == Some("Glass_Dark_01") {
                let material = materials.get_mut(material_handle).unwrap();
                material.alpha_mode = AlphaMode::Opaque;
                material.specular_transmission = 0.0;
            }
        }
    }
}

fn rotate_directional_light(
    mut animate_directional_light: Local<bool>,
    mut directional_light_transform: Single<&mut Transform, With<DirectionalLight>>,
    mut pathtracer: Option<Single<&mut Pathtracer>>,
    key_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if key_input.just_pressed(KeyCode::KeyL) {
        *animate_directional_light = !*animate_directional_light;
    }

    if *animate_directional_light {
        directional_light_transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_secs() * PI / 4.0,
            -std::f32::consts::FRAC_PI_4,
        );

        if let Some(pathtracer) = pathtracer.as_deref_mut() {
            pathtracer.reset = true;
        }
    } else {
        if let Some(pathtracer) = pathtracer.as_deref_mut() {
            pathtracer.reset = false;
        }
    }
}
