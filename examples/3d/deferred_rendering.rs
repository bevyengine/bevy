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
        .insert_resource(DefaultOpaqueRendererMethod(OpaqueRendererMethod::Deferred))
        .insert_resource(ClearColor(Color::rgb_linear(0.05, 0.05, 0.05)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_light_direction, switch_mode))
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
        FogSettings {
            color: Color::rgba(0.05, 0.05, 0.05, 1.0),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 8.0,
            },
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
        //NormalPrepass,
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
            maximum_distance: 10.0,
            ..default()
        }
        .into(),
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.0, -FRAC_PI_4)),
        ..default()
    });

    // FlightHelmet
    let helmet_scene = asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0");

    commands.spawn(SceneBundle {
        scene: helmet_scene.clone(),
        ..default()
    });
    commands.spawn(SceneBundle {
        scene: helmet_scene,
        transform: Transform::from_xyz(-3.0, 0.0, -3.0),
        ..default()
    });

    let mut forward_mat: StandardMaterial = Color::rgb(0.1, 0.2, 0.1).into();
    forward_mat.opaque_render_method = Some(OpaqueRendererMethod::Forward);
    let forward_mat_h = materials.add(forward_mat);

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.0).into()),
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

    // spheres
    for i in 0..6 {
        let j = i % 3;
        let s_val = if i < 3 { 0.0 } else { 0.2 };
        let material = if j == 0 {
            materials.add(StandardMaterial {
                base_color: Color::rgb(s_val, s_val, 1.0),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        } else if j == 1 {
            materials.add(StandardMaterial {
                base_color: Color::rgb(s_val, 1.0, s_val),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, s_val, s_val),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        };
        commands.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.125,
                sectors: 128,
                stacks: 128,
            })),
            material,
            transform: Transform::from_xyz(
                j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 } - 0.4,
                0.125,
                -j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 } + 0.4,
            ),
            ..default()
        });
    }
}

fn animate_light_direction(
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
    mut rotate: Local<bool>,
) {
    if input.just_pressed(KeyCode::Space) {
        *rotate = !*rotate;
    }
    if *rotate {
        for mut transform in &mut query {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.elapsed_seconds() * PI / 5.0,
                -FRAC_PI_4,
            );
        }
    }
}

fn switch_mode(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    mut default_opaque_renderer_method: ResMut<DefaultOpaqueRendererMethod>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cameras: Query<Entity, With<Camera>>,
) {
    if input.just_pressed(KeyCode::D) {
        default_opaque_renderer_method.0 = OpaqueRendererMethod::Deferred;
        println!("DefaultOpaqueRendererMethod: Deferred");
        for _ in materials.iter_mut() {}
        for camera in &cameras {
            commands.entity(camera).remove::<NormalPrepass>();
            commands.entity(camera).insert(DepthPrepass);
            commands.entity(camera).insert(MotionVectorPrepass);
            commands.entity(camera).insert(DeferredPrepass);
        }
    }
    if input.just_pressed(KeyCode::F) {
        default_opaque_renderer_method.0 = OpaqueRendererMethod::Forward;
        println!("DefaultOpaqueRendererMethod: Forward");
        for _ in materials.iter_mut() {}
        for camera in &cameras {
            commands.entity(camera).remove::<NormalPrepass>();
            commands.entity(camera).remove::<DepthPrepass>();
            commands.entity(camera).remove::<MotionVectorPrepass>();
            commands.entity(camera).remove::<DeferredPrepass>();
        }
    }
    if input.just_pressed(KeyCode::P) {
        default_opaque_renderer_method.0 = OpaqueRendererMethod::Forward;
        println!("DefaultOpaqueRendererMethod: Forward + Prepass");
        for _ in materials.iter_mut() {}
        for camera in &cameras {
            commands.entity(camera).insert(NormalPrepass);
            commands.entity(camera).insert(DepthPrepass);
            commands.entity(camera).insert(MotionVectorPrepass);
            commands.entity(camera).insert(DeferredPrepass);
        }
    }
}
