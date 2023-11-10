//! This example showcases light transmission
//!
//! ## Controls
//!
//! | Key Binding        | Action                                               |
//! |:-------------------|:-----------------------------------------------------|
//! | `J`/`K`/`L`/`;`    | Change Screen Space Transmission Quality             |
//! | `O` / `P`          | Decrease / Increase Screen Space Transmission Steps  |
//! | `1` / `2`          | Decrease / Increase Diffuse Transmission             |
//! | `Q` / `W`          | Decrease / Increase Specular Transmission            |
//! | `A` / `S`          | Decrease / Increase Thickness                        |
//! | `Z` / `X`          | Decrease / Increase IOR                              |
//! | `E` / `R`          | Decrease / Increase Perceptual Roughness             |
//! | `U` / `I`          | Decrease / Increase Reflectance                      |
//! | Arrow Keys         | Control Camera                                       |
//! | `C`                | Randomize Colors                                     |
//! | `H`                | Toggle HDR + Bloom                                   |
//! | `D`                | Toggle Depth Prepass                                 |
//! | `T`                | Toggle TAA                                           |

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use std::f32::consts::PI;

use bevy::{
    core_pipeline::{
        bloom::BloomSettings, core_3d::ScreenSpaceTransmissionQuality, prepass::DepthPrepass,
        tonemapping::Tonemapping,
    },
    pbr::{NotShadowCaster, PointLightShadowMap, TransmittedShadowReceiver},
    prelude::*,
    render::camera::TemporalJitter,
    render::view::ColorGrading,
};

#[cfg(not(all(feature = "webgl2", target_arch = "wasm32")))]
use bevy::core_pipeline::experimental::taa::{
    TemporalAntiAliasBundle, TemporalAntiAliasPlugin, TemporalAntiAliasSettings,
};

use rand::random;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PointLightShadowMap { size: 2048 })
        .insert_resource(AmbientLight {
            brightness: 0.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (example_control_system, flicker_system));

    // *Note:* TAA is not _required_ for specular transmission, but
    // it _greatly enhances_ the look of the resulting blur effects.
    // Sadly, it's not available under WebGL.
    #[cfg(not(all(feature = "webgl2", target_arch = "wasm32")))]
    app.insert_resource(Msaa::Off)
        .add_plugins(TemporalAntiAliasPlugin);

    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let icosphere_mesh = meshes.add(
        Mesh::try_from(shape::Icosphere {
            radius: 0.9,
            subdivisions: 7,
        })
        .unwrap(),
    );

    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.7 }));

    let plane_mesh = meshes.add(shape::Plane::from_size(2.0).into());

    let cylinder_mesh = meshes.add(
        Mesh::try_from(shape::Cylinder {
            radius: 0.5,
            height: 2.0,
            resolution: 50,
            segments: 1,
        })
        .unwrap(),
    );

    // Cube #1
    commands.spawn((
        PbrBundle {
            mesh: cube_mesh.clone(),
            material: materials.add(StandardMaterial { ..default() }),
            transform: Transform::from_xyz(0.25, 0.5, -2.0).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                1.4,
                3.7,
                21.3,
            )),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: false,
        },
    ));

    // Cube #2
    commands.spawn((
        PbrBundle {
            mesh: cube_mesh,
            material: materials.add(StandardMaterial { ..default() }),
            transform: Transform::from_xyz(-0.75, 0.7, -2.0).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                0.4,
                2.3,
                4.7,
            )),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: false,
        },
    ));

    // Candle
    commands.spawn((
        PbrBundle {
            mesh: cylinder_mesh,
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(0.9, 0.2, 0.3, 1.0),
                diffuse_transmission: 0.7,
                perceptual_roughness: 0.32,
                thickness: 0.2,
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: true,
        },
    ));

    // Candle Flame
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                emissive: Color::ANTIQUE_WHITE * 20.0 + Color::ORANGE_RED * 4.0,
                diffuse_transmission: 1.0,
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, 1.15, 0.0).with_scale(Vec3::new(0.1, 0.2, 0.1)),
            ..default()
        },
        Flicker,
        NotShadowCaster,
    ));

    // Glass Sphere
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                specular_transmission: 0.9,
                diffuse_transmission: 1.0,
                thickness: 1.8,
                ior: 1.5,
                perceptual_roughness: 0.12,
                ..default()
            }),
            transform: Transform::from_xyz(1.0, 0.0, 0.0),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // R Sphere
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::RED,
                specular_transmission: 0.9,
                diffuse_transmission: 1.0,
                thickness: 1.8,
                ior: 1.5,
                perceptual_roughness: 0.12,
                ..default()
            }),
            transform: Transform::from_xyz(1.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // G Sphere
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::GREEN,
                specular_transmission: 0.9,
                diffuse_transmission: 1.0,
                thickness: 1.8,
                ior: 1.5,
                perceptual_roughness: 0.12,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // B Sphere
    commands.spawn((
        PbrBundle {
            mesh: icosphere_mesh,
            material: materials.add(StandardMaterial {
                base_color: Color::BLUE,
                specular_transmission: 0.9,
                diffuse_transmission: 1.0,
                thickness: 1.8,
                ior: 1.5,
                perceptual_roughness: 0.12,
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
            ..default()
        },
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // Chessboard Plane
    let black_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        reflectance: 0.3,
        perceptual_roughness: 0.8,
        ..default()
    });

    let white_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        reflectance: 0.3,
        perceptual_roughness: 0.8,
        ..default()
    });

    for x in -3..4 {
        for z in -3..4 {
            commands.spawn((
                PbrBundle {
                    mesh: plane_mesh.clone(),
                    material: if (x + z) % 2 == 0 {
                        black_material.clone()
                    } else {
                        white_material.clone()
                    },
                    transform: Transform::from_xyz(x as f32 * 2.0, -1.0, z as f32 * 2.0),
                    ..default()
                },
                ExampleControls {
                    color: true,
                    specular_transmission: false,
                    diffuse_transmission: false,
                },
            ));
        }
    }

    // Paper
    commands.spawn((
        PbrBundle {
            mesh: plane_mesh,
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                diffuse_transmission: 0.6,
                perceptual_roughness: 0.8,
                reflectance: 1.0,
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, -3.0)
                .with_scale(Vec3::new(2.0, 1.0, 1.0))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, PI / 2.0, 0.0, 0.0)),
            ..default()
        },
        TransmittedShadowReceiver,
        ExampleControls {
            specular_transmission: false,
            color: false,
            diffuse_transmission: true,
        },
    ));

    // Candle Light
    commands.spawn((
        PointLightBundle {
            transform: Transform::from_xyz(-1.0, 1.7, 0.0),
            point_light: PointLight {
                color: Color::ANTIQUE_WHITE * 0.8 + Color::ORANGE_RED * 0.2,
                intensity: 1600.0,
                radius: 0.2,
                range: 5.0,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Flicker,
    ));

    // Camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(1.0, 1.8, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
            color_grading: ColorGrading {
                exposure: -2.0,
                post_saturation: 1.2,
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            ..default()
        },
        #[cfg(not(all(feature = "webgl2", target_arch = "wasm32")))]
        TemporalAntiAliasBundle::default(),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
        BloomSettings::default(),
    ));

    // Controls Text
    let text_style = TextStyle {
        font_size: 18.0,
        color: Color::WHITE,
        ..Default::default()
    };

    commands.spawn((
        TextBundle::from_section("", text_style).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        ExampleDisplay,
    ));
}

#[derive(Component)]
struct Flicker;

#[derive(Component)]
struct ExampleControls {
    diffuse_transmission: bool,
    specular_transmission: bool,
    color: bool,
}

struct ExampleState {
    diffuse_transmission: f32,
    specular_transmission: f32,
    thickness: f32,
    ior: f32,
    perceptual_roughness: f32,
    reflectance: f32,
    auto_camera: bool,
}

#[derive(Component)]
struct ExampleDisplay;

impl Default for ExampleState {
    fn default() -> Self {
        ExampleState {
            diffuse_transmission: 0.5,
            specular_transmission: 0.9,
            thickness: 1.8,
            ior: 1.5,
            perceptual_roughness: 0.12,
            reflectance: 0.5,
            auto_camera: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn example_control_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&Handle<StandardMaterial>, &ExampleControls)>,
    mut camera: Query<
        (
            Entity,
            &mut Camera,
            &mut Camera3d,
            &mut Transform,
            Option<&DepthPrepass>,
            Option<&TemporalJitter>,
        ),
        With<Camera3d>,
    >,
    mut display: Query<&mut Text, With<ExampleDisplay>>,
    mut state: Local<ExampleState>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    if input.pressed(KeyCode::Key2) {
        state.diffuse_transmission = (state.diffuse_transmission + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::Key1) {
        state.diffuse_transmission = (state.diffuse_transmission - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::W) {
        state.specular_transmission = (state.specular_transmission + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::Q) {
        state.specular_transmission = (state.specular_transmission - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::S) {
        state.thickness = (state.thickness + time.delta_seconds()).min(5.0);
    } else if input.pressed(KeyCode::A) {
        state.thickness = (state.thickness - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::X) {
        state.ior = (state.ior + time.delta_seconds()).min(3.0);
    } else if input.pressed(KeyCode::Z) {
        state.ior = (state.ior - time.delta_seconds()).max(1.0);
    }

    if input.pressed(KeyCode::I) {
        state.reflectance = (state.reflectance + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::U) {
        state.reflectance = (state.reflectance - time.delta_seconds()).max(0.0);
    }

    if input.pressed(KeyCode::R) {
        state.perceptual_roughness = (state.perceptual_roughness + time.delta_seconds()).min(1.0);
    } else if input.pressed(KeyCode::E) {
        state.perceptual_roughness = (state.perceptual_roughness - time.delta_seconds()).max(0.0);
    }

    let randomize_colors = input.just_pressed(KeyCode::C);

    for (material_handle, controls) in &controllable {
        let material = materials.get_mut(material_handle).unwrap();
        if controls.specular_transmission {
            material.specular_transmission = state.specular_transmission;
            material.thickness = state.thickness;
            material.ior = state.ior;
            material.perceptual_roughness = state.perceptual_roughness;
            material.reflectance = state.reflectance;
        }

        if controls.diffuse_transmission {
            material.diffuse_transmission = state.diffuse_transmission;
        }

        if controls.color && randomize_colors {
            material.base_color.set_r(random());
            material.base_color.set_g(random());
            material.base_color.set_b(random());
        }
    }

    let (
        camera_entity,
        mut camera,
        mut camera_3d,
        mut camera_transform,
        depth_prepass,
        temporal_jitter,
    ) = camera.single_mut();

    if input.just_pressed(KeyCode::H) {
        camera.hdr = !camera.hdr;
    }

    #[cfg(not(all(feature = "webgl2", target_arch = "wasm32")))]
    if input.just_pressed(KeyCode::D) {
        if depth_prepass.is_none() {
            commands.entity(camera_entity).insert(DepthPrepass);
        } else {
            commands.entity(camera_entity).remove::<DepthPrepass>();
        }
    }

    #[cfg(not(all(feature = "webgl2", target_arch = "wasm32")))]
    if input.just_pressed(KeyCode::T) {
        if temporal_jitter.is_none() {
            commands.entity(camera_entity).insert((
                TemporalJitter::default(),
                TemporalAntiAliasSettings::default(),
            ));
        } else {
            commands
                .entity(camera_entity)
                .remove::<(TemporalJitter, TemporalAntiAliasSettings)>();
        }
    }

    if input.just_pressed(KeyCode::O) && camera_3d.screen_space_specular_transmission_steps > 0 {
        camera_3d.screen_space_specular_transmission_steps -= 1;
    }

    if input.just_pressed(KeyCode::P) && camera_3d.screen_space_specular_transmission_steps < 4 {
        camera_3d.screen_space_specular_transmission_steps += 1;
    }

    if input.just_pressed(KeyCode::J) {
        camera_3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
    }

    if input.just_pressed(KeyCode::K) {
        camera_3d.screen_space_specular_transmission_quality =
            ScreenSpaceTransmissionQuality::Medium;
    }

    if input.just_pressed(KeyCode::L) {
        camera_3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::High;
    }

    if input.just_pressed(KeyCode::Semicolon) {
        camera_3d.screen_space_specular_transmission_quality =
            ScreenSpaceTransmissionQuality::Ultra;
    }

    let rotation = if input.pressed(KeyCode::Right) {
        state.auto_camera = false;
        time.delta_seconds()
    } else if input.pressed(KeyCode::Left) {
        state.auto_camera = false;
        -time.delta_seconds()
    } else if state.auto_camera {
        time.delta_seconds() * 0.25
    } else {
        0.0
    };

    let distance_change =
        if input.pressed(KeyCode::Down) && camera_transform.translation.length() < 25.0 {
            time.delta_seconds()
        } else if input.pressed(KeyCode::Up) && camera_transform.translation.length() > 2.0 {
            -time.delta_seconds()
        } else {
            0.0
        };

    camera_transform.translation *= distance_change.exp();

    camera_transform.rotate_around(
        Vec3::ZERO,
        Quat::from_euler(EulerRot::XYZ, 0.0, rotation, 0.0),
    );

    let mut display = display.single_mut();
    display.sections[0].value = format!(
        concat!(
            " J / K / L / ;  Screen Space Specular Transmissive Quality: {:?}\n",
            "         O / P  Screen Space Specular Transmissive Steps: {}\n",
            "         1 / 2  Diffuse Transmission: {:.2}\n",
            "         Q / W  Specular Transmission: {:.2}\n",
            "         A / S  Thickness: {:.2}\n",
            "         Z / X  IOR: {:.2}\n",
            "         E / R  Perceptual Roughness: {:.2}\n",
            "         U / I  Reflectance: {:.2}\n",
            "    Arrow Keys  Control Camera\n",
            "             C  Randomize Colors\n",
            "             H  HDR + Bloom: {}\n",
            "             D  Depth Prepass: {}\n",
            "             T  TAA: {}\n",
        ),
        camera_3d.screen_space_specular_transmission_quality,
        camera_3d.screen_space_specular_transmission_steps,
        state.diffuse_transmission,
        state.specular_transmission,
        state.thickness,
        state.ior,
        state.perceptual_roughness,
        state.reflectance,
        if camera.hdr { "ON " } else { "OFF" },
        if cfg!(any(not(feature = "webgl2"), not(target_arch = "wasm32"))) {
            if depth_prepass.is_some() {
                "ON "
            } else {
                "OFF"
            }
        } else {
            "N/A (WebGL)"
        },
        if cfg!(any(not(feature = "webgl2"), not(target_arch = "wasm32"))) {
            if temporal_jitter.is_some() {
                if depth_prepass.is_some() {
                    "ON "
                } else {
                    "N/A (Needs Depth Prepass)"
                }
            } else {
                "OFF"
            }
        } else {
            "N/A (WebGL)"
        },
    );
}

fn flicker_system(
    mut flame: Query<&mut Transform, (With<Flicker>, With<Handle<Mesh>>)>,
    mut light: Query<(&mut PointLight, &mut Transform), (With<Flicker>, Without<Handle<Mesh>>)>,
    time: Res<Time>,
) {
    let s = time.elapsed_seconds();
    let a = (s * 6.0).cos() * 0.0125 + (s * 4.0).cos() * 0.025;
    let b = (s * 5.0).cos() * 0.0125 + (s * 3.0).cos() * 0.025;
    let c = (s * 7.0).cos() * 0.0125 + (s * 2.0).cos() * 0.025;
    let (mut light, mut light_transform) = light.single_mut();
    let mut flame_transform = flame.single_mut();
    light.intensity = 1600.0 + 3000.0 * (a + b + c);
    flame_transform.translation = Vec3::new(-1.0, 1.23, 0.0);
    flame_transform.look_at(Vec3::new(-1.0 - c, 1.7 - b, 0.0 - a), Vec3::X);
    flame_transform.rotate(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, PI / 2.0));
    light_transform.translation = Vec3::new(-1.0 - c, 1.7, 0.0 - a);
    flame_transform.translation = Vec3::new(-1.0 - c, 1.23, 0.0 - a);
}
