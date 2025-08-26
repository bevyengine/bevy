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

use std::f32::consts::PI;

use bevy::{
    camera::{Exposure, ScreenSpaceTransmissionQuality},
    color::palettes::css::*,
    core_pipeline::{bloom::Bloom, prepass::DepthPrepass, tonemapping::Tonemapping},
    light::{NotShadowCaster, PointLightShadowMap, TransmittedShadowReceiver},
    math::ops,
    prelude::*,
    render::{
        camera::TemporalJitter,
        view::{ColorGrading, ColorGradingGlobal, Hdr},
    },
};

// *Note:* TAA is not _required_ for specular transmission, but
// it _greatly enhances_ the look of the resulting blur effects.
// Sadly, it's not available under WebGL.
#[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
use bevy::anti_aliasing::taa::TemporalAntiAliasing;

use rand::random;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PointLightShadowMap { size: 2048 })
        .insert_resource(AmbientLight {
            brightness: 0.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (example_control_system, flicker_system))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let icosphere_mesh = meshes.add(Sphere::new(0.9).mesh().ico(7).unwrap());
    let cube_mesh = meshes.add(Cuboid::new(0.7, 0.7, 0.7));
    let plane_mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
    let cylinder_mesh = meshes.add(Cylinder::new(0.5, 2.0).mesh().resolution(50));

    // Cube #1
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_xyz(0.25, 0.5, -2.0).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            1.4,
            3.7,
            21.3,
        )),
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: false,
        },
    ));

    // Cube #2
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_xyz(-0.75, 0.7, -2.0).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            0.4,
            2.3,
            4.7,
        )),
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: false,
        },
    ));

    // Candle
    commands.spawn((
        Mesh3d(cylinder_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.2, 0.3),
            diffuse_transmission: 0.7,
            perceptual_roughness: 0.32,
            thickness: 0.2,
            ..default()
        })),
        Transform::from_xyz(-1.0, 0.0, 0.0),
        ExampleControls {
            color: true,
            specular_transmission: false,
            diffuse_transmission: true,
        },
    ));

    // Candle Flame
    let scaled_white = LinearRgba::from(ANTIQUE_WHITE) * 20.;
    let scaled_orange = LinearRgba::from(ORANGE_RED) * 4.;
    let emissive = LinearRgba {
        red: scaled_white.red + scaled_orange.red,
        green: scaled_white.green + scaled_orange.green,
        blue: scaled_white.blue + scaled_orange.blue,
        alpha: 1.0,
    };

    commands.spawn((
        Mesh3d(icosphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            emissive,
            diffuse_transmission: 1.0,
            ..default()
        })),
        Transform::from_xyz(-1.0, 1.15, 0.0).with_scale(Vec3::new(0.1, 0.2, 0.1)),
        Flicker,
        NotShadowCaster,
    ));

    // Glass Sphere
    commands.spawn((
        Mesh3d(icosphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            specular_transmission: 0.9,
            diffuse_transmission: 1.0,
            thickness: 1.8,
            ior: 1.5,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(1.0, 0.0, 0.0),
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // R Sphere
    commands.spawn((
        Mesh3d(icosphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.into(),
            specular_transmission: 0.9,
            diffuse_transmission: 1.0,
            thickness: 1.8,
            ior: 1.5,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(1.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // G Sphere
    commands.spawn((
        Mesh3d(icosphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LIME.into(),
            specular_transmission: 0.9,
            diffuse_transmission: 1.0,
            thickness: 1.8,
            ior: 1.5,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
        ExampleControls {
            color: true,
            specular_transmission: true,
            diffuse_transmission: false,
        },
    ));

    // B Sphere
    commands.spawn((
        Mesh3d(icosphere_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.into(),
            specular_transmission: 0.9,
            diffuse_transmission: 1.0,
            thickness: 1.8,
            ior: 1.5,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(-1.0, -0.5, 2.0).with_scale(Vec3::splat(0.5)),
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
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(if (x + z) % 2 == 0 {
                    black_material.clone()
                } else {
                    white_material.clone()
                }),
                Transform::from_xyz(x as f32 * 2.0, -1.0, z as f32 * 2.0),
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
        Mesh3d(plane_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            diffuse_transmission: 0.6,
            perceptual_roughness: 0.8,
            reflectance: 1.0,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, -3.0)
            .with_scale(Vec3::new(2.0, 1.0, 1.0))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, PI / 2.0, 0.0, 0.0)),
        TransmittedShadowReceiver,
        ExampleControls {
            specular_transmission: false,
            color: false,
            diffuse_transmission: true,
        },
    ));

    // Candle Light
    commands.spawn((
        Transform::from_xyz(-1.0, 1.7, 0.0),
        PointLight {
            color: Color::from(
                LinearRgba::from(ANTIQUE_WHITE).mix(&LinearRgba::from(ORANGE_RED), 0.2),
            ),
            intensity: 4_000.0,
            radius: 0.2,
            range: 5.0,
            shadows_enabled: true,
            ..default()
        },
        Flicker,
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.0, 1.8, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
        ColorGrading {
            global: ColorGradingGlobal {
                post_saturation: 1.2,
                ..default()
            },
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Exposure { ev100: 6.0 },
        #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
        Msaa::Off,
        #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
        TemporalAntiAliasing::default(),
        EnvironmentMapLight {
            intensity: 25.0,
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            ..default()
        },
        Bloom::default(),
    ));

    // Controls Text
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
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

fn example_control_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    controllable: Query<(&MeshMaterial3d<StandardMaterial>, &ExampleControls)>,
    camera: Single<
        (
            Entity,
            &mut Camera3d,
            &mut Transform,
            Option<&DepthPrepass>,
            Option<&TemporalJitter>,
            Has<Hdr>,
        ),
        With<Camera3d>,
    >,
    mut display: Single<&mut Text, With<ExampleDisplay>>,
    mut state: Local<ExampleState>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::Digit2) {
        state.diffuse_transmission = (state.diffuse_transmission + time.delta_secs()).min(1.0);
    } else if input.pressed(KeyCode::Digit1) {
        state.diffuse_transmission = (state.diffuse_transmission - time.delta_secs()).max(0.0);
    }

    if input.pressed(KeyCode::KeyW) {
        state.specular_transmission = (state.specular_transmission + time.delta_secs()).min(1.0);
    } else if input.pressed(KeyCode::KeyQ) {
        state.specular_transmission = (state.specular_transmission - time.delta_secs()).max(0.0);
    }

    if input.pressed(KeyCode::KeyS) {
        state.thickness = (state.thickness + time.delta_secs()).min(5.0);
    } else if input.pressed(KeyCode::KeyA) {
        state.thickness = (state.thickness - time.delta_secs()).max(0.0);
    }

    if input.pressed(KeyCode::KeyX) {
        state.ior = (state.ior + time.delta_secs()).min(3.0);
    } else if input.pressed(KeyCode::KeyZ) {
        state.ior = (state.ior - time.delta_secs()).max(1.0);
    }

    if input.pressed(KeyCode::KeyI) {
        state.reflectance = (state.reflectance + time.delta_secs()).min(1.0);
    } else if input.pressed(KeyCode::KeyU) {
        state.reflectance = (state.reflectance - time.delta_secs()).max(0.0);
    }

    if input.pressed(KeyCode::KeyR) {
        state.perceptual_roughness = (state.perceptual_roughness + time.delta_secs()).min(1.0);
    } else if input.pressed(KeyCode::KeyE) {
        state.perceptual_roughness = (state.perceptual_roughness - time.delta_secs()).max(0.0);
    }

    let randomize_colors = input.just_pressed(KeyCode::KeyC);

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
            material.base_color =
                Color::srgba(random(), random(), random(), material.base_color.alpha());
        }
    }

    let (camera_entity, mut camera_3d, mut camera_transform, depth_prepass, temporal_jitter, hdr) =
        camera.into_inner();

    if input.just_pressed(KeyCode::KeyH) {
        if hdr {
            commands.entity(camera_entity).remove::<Hdr>();
        } else {
            commands.entity(camera_entity).insert(Hdr);
        }
    }

    #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
    if input.just_pressed(KeyCode::KeyD) {
        if depth_prepass.is_none() {
            commands.entity(camera_entity).insert(DepthPrepass);
        } else {
            commands.entity(camera_entity).remove::<DepthPrepass>();
        }
    }

    #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
    if input.just_pressed(KeyCode::KeyT) {
        if temporal_jitter.is_none() {
            commands
                .entity(camera_entity)
                .insert((TemporalJitter::default(), TemporalAntiAliasing::default()));
        } else {
            commands
                .entity(camera_entity)
                .remove::<(TemporalJitter, TemporalAntiAliasing)>();
        }
    }

    if input.just_pressed(KeyCode::KeyO) && camera_3d.screen_space_specular_transmission_steps > 0 {
        camera_3d.screen_space_specular_transmission_steps -= 1;
    }

    if input.just_pressed(KeyCode::KeyP) && camera_3d.screen_space_specular_transmission_steps < 4 {
        camera_3d.screen_space_specular_transmission_steps += 1;
    }

    if input.just_pressed(KeyCode::KeyJ) {
        camera_3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
    }

    if input.just_pressed(KeyCode::KeyK) {
        camera_3d.screen_space_specular_transmission_quality =
            ScreenSpaceTransmissionQuality::Medium;
    }

    if input.just_pressed(KeyCode::KeyL) {
        camera_3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::High;
    }

    if input.just_pressed(KeyCode::Semicolon) {
        camera_3d.screen_space_specular_transmission_quality =
            ScreenSpaceTransmissionQuality::Ultra;
    }

    let rotation = if input.pressed(KeyCode::ArrowRight) {
        state.auto_camera = false;
        time.delta_secs()
    } else if input.pressed(KeyCode::ArrowLeft) {
        state.auto_camera = false;
        -time.delta_secs()
    } else if state.auto_camera {
        time.delta_secs() * 0.25
    } else {
        0.0
    };

    let distance_change =
        if input.pressed(KeyCode::ArrowDown) && camera_transform.translation.length() < 25.0 {
            time.delta_secs()
        } else if input.pressed(KeyCode::ArrowUp) && camera_transform.translation.length() > 2.0 {
            -time.delta_secs()
        } else {
            0.0
        };

    camera_transform.translation *= ops::exp(distance_change);

    camera_transform.rotate_around(
        Vec3::ZERO,
        Quat::from_euler(EulerRot::XYZ, 0.0, rotation, 0.0),
    );

    display.0 = format!(
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
        if hdr { "ON " } else { "OFF" },
        if cfg!(any(feature = "webgpu", not(target_arch = "wasm32"))) {
            if depth_prepass.is_some() {
                "ON "
            } else {
                "OFF"
            }
        } else {
            "N/A (WebGL)"
        },
        if cfg!(any(feature = "webgpu", not(target_arch = "wasm32"))) {
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
    mut flame: Single<&mut Transform, (With<Flicker>, With<Mesh3d>)>,
    light: Single<(&mut PointLight, &mut Transform), (With<Flicker>, Without<Mesh3d>)>,
    time: Res<Time>,
) {
    let s = time.elapsed_secs();
    let a = ops::cos(s * 6.0) * 0.0125 + ops::cos(s * 4.0) * 0.025;
    let b = ops::cos(s * 5.0) * 0.0125 + ops::cos(s * 3.0) * 0.025;
    let c = ops::cos(s * 7.0) * 0.0125 + ops::cos(s * 2.0) * 0.025;
    let (mut light, mut light_transform) = light.into_inner();
    light.intensity = 4_000.0 + 3000.0 * (a + b + c);
    flame.translation = Vec3::new(-1.0, 1.23, 0.0);
    flame.look_at(Vec3::new(-1.0 - c, 1.7 - b, 0.0 - a), Vec3::X);
    flame.rotate(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, PI / 2.0));
    light_transform.translation = Vec3::new(-1.0 - c, 1.7, 0.0 - a);
    flame.translation = Vec3::new(-1.0 - c, 1.23, 0.0 - a);
}
