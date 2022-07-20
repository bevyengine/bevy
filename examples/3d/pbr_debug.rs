//! Loads and renders a glTF file as a scene and displays rendering debug visualizations.

use bevy::{pbr::PbrDebug, prelude::*};

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(animate_light_direction)
        .add_system(cycle_pbr_debug)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..default()
    });
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
    commands.spawn_bundle(SceneBundle {
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
        ..default()
    });
}

fn cycle_pbr_debug(
    time: Res<Time>,
    mut next_switch: Local<Option<f64>>,
    mut pbr_debug: ResMut<PbrDebug>,
) {
    let now = time.seconds_since_startup();
    if next_switch.is_none() {
        *next_switch = Some(now + 2.0);
    }
    if now > next_switch.unwrap() {
        *pbr_debug = match *pbr_debug {
            PbrDebug::None => PbrDebug::Uvs,
            PbrDebug::Uvs => PbrDebug::Depth,
            PbrDebug::Depth => PbrDebug::InterpolatedVertexNormals,
            PbrDebug::InterpolatedVertexNormals => PbrDebug::InterpolatedVertexTangents,
            PbrDebug::InterpolatedVertexTangents => PbrDebug::TangentSpaceNormalMap,
            PbrDebug::TangentSpaceNormalMap => PbrDebug::NormalMappedNormal,
            PbrDebug::NormalMappedNormal => PbrDebug::ViewSpaceNormalMappedNormal,
            PbrDebug::ViewSpaceNormalMappedNormal => PbrDebug::BaseColor,
            PbrDebug::BaseColor => PbrDebug::BaseColorTexture,
            PbrDebug::BaseColorTexture => PbrDebug::Emissive,
            PbrDebug::Emissive => PbrDebug::EmissiveTexture,
            PbrDebug::EmissiveTexture => PbrDebug::Roughness,
            PbrDebug::Roughness => PbrDebug::RoughnessTexture,
            PbrDebug::RoughnessTexture => PbrDebug::Metallic,
            PbrDebug::Metallic => PbrDebug::MetallicTexture,
            PbrDebug::MetallicTexture => PbrDebug::Reflectance,
            PbrDebug::Reflectance => PbrDebug::OcclusionTexture,
            PbrDebug::OcclusionTexture => PbrDebug::Opaque,
            PbrDebug::Opaque => PbrDebug::AlphaMask,
            PbrDebug::AlphaMask => PbrDebug::AlphaBlend,
            PbrDebug::AlphaBlend => PbrDebug::ClusteredForwardDebugZSlices,
            PbrDebug::ClusteredForwardDebugZSlices => {
                PbrDebug::ClusteredForwardDebugClusterLightComplexity
            }
            PbrDebug::ClusteredForwardDebugClusterLightComplexity => {
                PbrDebug::ClusteredForwardDebugClusterCoherency
            }
            PbrDebug::ClusteredForwardDebugClusterCoherency => PbrDebug::None,
        };
        info!("Switching to {:?}", *pbr_debug);
        *next_switch = Some(next_switch.unwrap() + 2.0);
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 10.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}
