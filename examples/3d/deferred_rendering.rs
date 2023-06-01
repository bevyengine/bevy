//! Loads and renders a glTF file as a scene.

use std::f32::consts::*;

use bevy::{
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};
use bevy_internal::{
    core_pipeline::{
        fxaa::Fxaa,
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    },
    pbr::{DefaultOpaqueRendererMethod, OpaqueRendererMethod},
};

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .insert_resource(DefaultOpaqueRendererMethod(OpaqueRendererMethod::Deferred))
        .add_systems(Startup, setup)
        .add_systems(Update, animate_light_direction)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                //hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
        NormalPrepass,
        DepthPrepass,
        MotionVectorPrepass,
        DeferredPrepass,
        Fxaa::default(),
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 3,
            maximum_distance: 5.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // FlightHelmet
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
        ..default()
    });

    let mut forward_mat: StandardMaterial = Color::rgb(0.3, 0.5, 0.3).into();
    forward_mat.opaque_render_method = Some(OpaqueRendererMethod::Forward);
    let forward_mat_h = materials.add(forward_mat);

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: forward_mat_h.clone(),
        ..default()
    });

    let cube_h = meshes.add(Mesh::from(shape::Cube { size: 0.1 }));

    // cubes
    commands.spawn(PbrBundle {
        mesh: cube_h.clone(),
        material: forward_mat_h.clone(),
        transform: Transform::from_xyz(-0.3, 0.5, -0.2),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: cube_h,
        material: forward_mat_h,
        transform: Transform::from_xyz(0.2, 0.5, 0.2),
        ..default()
    });
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_seconds() * PI / 5.0,
            -FRAC_PI_4,
        );
    }
}
