//! This example compares MSAA (Multi-Sample Anti-aliasing), FXAA (Fast Approximate Anti-aliasing), and TAA (Temporal Anti-aliasing).
//!
//! Add the `--screenshot_taa` to save a TAA motion test screenshot.

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::math::uvec2;
use bevy::{
    core_pipeline::{
        contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings,
        experimental::taa::{
            TemporalAntiAliasBundle, TemporalAntiAliasPlugin, TemporalAntiAliasSettings,
        },
        fxaa::{Fxaa, Sensitivity},
    },
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        render_resource::{Extent3d, SamplerDescriptor, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
};
use bevy::{
    math::vec3,
    render::{
        mesh::VertexAttributeValues,
        render_resource::{AddressMode, FilterMode},
    },
};

use bevy_internal::app::AppExit;
use bevy_internal::core::FrameCount;
use bevy_internal::render::view::screenshot::ScreenshotManager;
use bevy_internal::window::{PrimaryWindow, WindowResized};
use image::imageops::FilterType;

#[derive(Resource)]
struct ScreenshotTaa;

fn main() {
    let mut app = App::new();
    let mut movement_settings = CameraMovementSettings::default();
    if std::env::args().nth(1).unwrap_or_default() == "--screenshot_taa" {
        app.insert_resource(ScreenshotTaa);
        app.add_systems(PostUpdate, screenshot);
        movement_settings.rotate_camera = true;
        movement_settings.circle_look_camera = true;
    }
    app.insert_resource(Msaa::Off)
        .insert_resource(movement_settings)
        .add_plugins((DefaultPlugins, TemporalAntiAliasPlugin))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                modify_aa,
                modify_sharpening,
                update_ui,
                update_uv_scale,
                rotate_camera,
            ),
        )
        .run();
}

fn screenshot(
    main_window: Query<Entity, With<PrimaryWindow>>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut counter: Local<u32>,
    frame_count: Res<FrameCount>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if frame_count.0 == 436 {
        let path = format!("./taa_screenshot-{}.png", *counter);
        *counter += 1;
        screenshot_manager
            .save_screenshot_to_disk(main_window.single(), path)
            .unwrap();
    }
    if frame_count.0 == 600 {
        app_exit_events.send(AppExit);
    }
}

#[derive(Resource, Default)]
struct CameraMovementSettings {
    rotate_camera: bool,
    circle_look_camera: bool,
}

fn modify_aa(
    keys: Res<Input<KeyCode>>,
    mut camera: Query<
        (
            Entity,
            Option<&mut Fxaa>,
            Option<&TemporalAntiAliasSettings>,
        ),
        With<Camera>,
    >,
    mut msaa: ResMut<Msaa>,
    mut commands: Commands,
    mut camera_movement_settings: ResMut<CameraMovementSettings>,
) {
    let (camera_entity, fxaa, taa) = camera.single_mut();
    let mut camera = commands.entity(camera_entity);

    // No AA
    if keys.just_pressed(KeyCode::Key1) {
        *msaa = Msaa::Off;
        camera.remove::<Fxaa>();
        camera.remove::<TemporalAntiAliasBundle>();
    }

    // MSAA
    if keys.just_pressed(KeyCode::Key2) && *msaa == Msaa::Off {
        camera.remove::<Fxaa>();
        camera.remove::<TemporalAntiAliasBundle>();

        *msaa = Msaa::Sample4;
    }

    // MSAA Sample Count
    if *msaa != Msaa::Off {
        if keys.just_pressed(KeyCode::Q) {
            *msaa = Msaa::Sample2;
        }
        if keys.just_pressed(KeyCode::W) {
            *msaa = Msaa::Sample4;
        }
        if keys.just_pressed(KeyCode::E) {
            *msaa = Msaa::Sample8;
        }
    }

    // FXAA
    if keys.just_pressed(KeyCode::Key3) && fxaa.is_none() {
        *msaa = Msaa::Off;
        camera.remove::<TemporalAntiAliasBundle>();

        camera.insert(Fxaa::default());
    }

    // FXAA Settings
    if let Some(mut fxaa) = fxaa {
        if keys.just_pressed(KeyCode::Q) {
            fxaa.edge_threshold = Sensitivity::Low;
            fxaa.edge_threshold_min = Sensitivity::Low;
        }
        if keys.just_pressed(KeyCode::W) {
            fxaa.edge_threshold = Sensitivity::Medium;
            fxaa.edge_threshold_min = Sensitivity::Medium;
        }
        if keys.just_pressed(KeyCode::E) {
            fxaa.edge_threshold = Sensitivity::High;
            fxaa.edge_threshold_min = Sensitivity::High;
        }
        if keys.just_pressed(KeyCode::R) {
            fxaa.edge_threshold = Sensitivity::Ultra;
            fxaa.edge_threshold_min = Sensitivity::Ultra;
        }
        if keys.just_pressed(KeyCode::T) {
            fxaa.edge_threshold = Sensitivity::Extreme;
            fxaa.edge_threshold_min = Sensitivity::Extreme;
        }
    }

    // TAA
    if keys.just_pressed(KeyCode::Key4) && taa.is_none() {
        *msaa = Msaa::Off;
        camera.remove::<Fxaa>();

        camera.insert(TemporalAntiAliasBundle::default());
    }

    // Rotate Camera
    if keys.just_pressed(KeyCode::K) {
        camera_movement_settings.rotate_camera = !camera_movement_settings.rotate_camera;
    }

    // Circle look camera
    if keys.just_pressed(KeyCode::L) {
        camera_movement_settings.circle_look_camera = !camera_movement_settings.circle_look_camera;
    }
}

fn modify_sharpening(
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut ContrastAdaptiveSharpeningSettings>,
) {
    for mut cas in &mut query {
        if keys.just_pressed(KeyCode::Key0) {
            cas.enabled = !cas.enabled;
        }
        if cas.enabled {
            if keys.just_pressed(KeyCode::Minus) {
                cas.sharpening_strength -= 0.1;
                cas.sharpening_strength = cas.sharpening_strength.clamp(0.0, 1.0);
            }
            if keys.just_pressed(KeyCode::Equals) {
                cas.sharpening_strength += 0.1;
                cas.sharpening_strength = cas.sharpening_strength.clamp(0.0, 1.0);
            }
            if keys.just_pressed(KeyCode::D) {
                cas.denoise = !cas.denoise;
            }
        }
    }
}

fn update_ui(
    mut camera: Query<
        (
            Option<&Fxaa>,
            Option<&TemporalAntiAliasSettings>,
            &ContrastAdaptiveSharpeningSettings,
        ),
        With<Camera>,
    >,
    msaa: Res<Msaa>,
    mut ui: Query<&mut Text>,
    camera_movement_settings: Res<CameraMovementSettings>,
) {
    let (fxaa, taa, cas_settings) = camera.single_mut();

    let mut ui = ui.single_mut();
    let ui = &mut ui.sections[0].value;

    *ui = "Antialias Method\n".to_string();

    if *msaa == Msaa::Off && fxaa.is_none() && taa.is_none() {
        ui.push_str("(1) *No AA*\n");
    } else {
        ui.push_str("(1) No AA\n");
    }

    if *msaa != Msaa::Off {
        ui.push_str("(2) *MSAA*\n");
    } else {
        ui.push_str("(2) MSAA\n");
    }

    if fxaa.is_some() {
        ui.push_str("(3) *FXAA*\n");
    } else {
        ui.push_str("(3) FXAA\n");
    }

    if taa.is_some() {
        ui.push_str("(4) *TAA*");
    } else {
        ui.push_str("(4) TAA");
    }

    if *msaa != Msaa::Off {
        ui.push_str("\n\n----------\n\nSample Count\n");

        if *msaa == Msaa::Sample2 {
            ui.push_str("(Q) *2*\n");
        } else {
            ui.push_str("(Q) 2\n");
        }
        if *msaa == Msaa::Sample4 {
            ui.push_str("(W) *4*\n");
        } else {
            ui.push_str("(W) 4\n");
        }
        if *msaa == Msaa::Sample8 {
            ui.push_str("(E) *8*");
        } else {
            ui.push_str("(E) 8");
        }
    }

    if let Some(fxaa) = fxaa {
        ui.push_str("\n\n----------\n\nSensitivity\n");

        if fxaa.edge_threshold == Sensitivity::Low {
            ui.push_str("(Q) *Low*\n");
        } else {
            ui.push_str("(Q) Low\n");
        }

        if fxaa.edge_threshold == Sensitivity::Medium {
            ui.push_str("(W) *Medium*\n");
        } else {
            ui.push_str("(W) Medium\n");
        }

        if fxaa.edge_threshold == Sensitivity::High {
            ui.push_str("(E) *High*\n");
        } else {
            ui.push_str("(E) High\n");
        }

        if fxaa.edge_threshold == Sensitivity::Ultra {
            ui.push_str("(R) *Ultra*\n");
        } else {
            ui.push_str("(R) Ultra\n");
        }

        if fxaa.edge_threshold == Sensitivity::Extreme {
            ui.push_str("(T) *Extreme*");
        } else {
            ui.push_str("(T) Extreme");
        }
    }

    if cas_settings.enabled {
        ui.push_str("\n\n----------\n\n(0) Sharpening (Enabled)\n");
        ui.push_str(&format!(
            "(-/+) Strength: {:.1}\n",
            cas_settings.sharpening_strength
        ));
        if cas_settings.denoise {
            ui.push_str("(D) Denoising (Enabled)\n");
        } else {
            ui.push_str("(D) Denoising (Disabled)\n");
        }
    } else {
        ui.push_str("\n\n----------\n\n(0) Sharpening (Disabled)\n");
    }

    ui.push_str("\n----------\n\n");

    if camera_movement_settings.rotate_camera {
        ui.push_str("(K) *Rotate Camera*\n");
    } else {
        ui.push_str("(K) Rotate Camera\n");
    }

    if camera_movement_settings.circle_look_camera {
        ui.push_str("(L) *Look in circle*\n");
    } else {
        ui.push_str("(L) Rotate Camera\n");
    }
}

fn rotate_camera(
    time: Res<Time>,
    frame_count: Res<FrameCount>,
    mut camera: Query<&mut Transform, With<Camera>>,
    camera_movement_settings: Res<CameraMovementSettings>,
    screenshot_taa: Option<Res<ScreenshotTaa>>,
) {
    let elapsed_time = if screenshot_taa.is_some() {
        // Use a fix time step for animation when taking screenshot so TAA and movement is consistent across frame rates
        frame_count.0 as f32 / 60.0
    } else {
        time.elapsed_seconds()
    };

    let mut transform = camera.single_mut();
    if camera_movement_settings.rotate_camera {
        let speed = 1.0;
        let t = (elapsed_time * speed) % TAU;
        let radius = 2.0;
        transform.translation = vec3(t.cos() * radius, 0.5, t.sin() * radius);
    }

    if camera_movement_settings.circle_look_camera {
        let speed = 5.0;
        let t = (elapsed_time * speed) % TAU;
        let radius = 0.3;
        transform.look_at(
            vec3(t.cos() * radius, 0.2, t.sin() * radius),
            vec3(0.0, 1.0, 0.0),
        );
    } else {
        transform.look_at(vec3(0.0, 0.2, 0.0), vec3(0.0, 1.0, 0.0));
    }
}

#[derive(Resource)]
struct NoisePlaneMesh(Handle<Mesh>);

/// Set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    main_window: Query<&Window, With<PrimaryWindow>>,
    screenshot_taa: Option<Res<ScreenshotTaa>>,
) {
    let checker_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let noise_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(noise_debug_texture())),
        unlit: true,
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // Plane 2
    let mut noise_mesh: Mesh = shape::Plane::from_size(5.0).into();

    let height = main_window.single().resolution.physical_height();

    set_noise_mesh_uv_scale(height, &mut noise_mesh);

    let mesh_h = meshes.add(noise_mesh);
    commands.insert_resource(NoisePlaneMesh(mesh_h.clone()));

    commands.spawn(PbrBundle {
        mesh: mesh_h,
        material: noise_material,
        transform: Transform::from_xyz(0.0, 1.0, -1.5).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            FRAC_PI_2,
            0.0,
            0.0,
        )),
        ..default()
    });

    let cube_h = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));

    // Cubes
    for i in 0..5 {
        commands.spawn(PbrBundle {
            mesh: cube_h.clone(),
            material: checker_material.clone(),
            transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
            ..default()
        });
    }

    let helmet = asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0");

    // Flight Helmet
    commands.spawn(SceneBundle {
        scene: helmet.clone(),
        ..default()
    });
    commands.spawn(SceneBundle {
        scene: helmet,
        transform: Transform::from_translation(vec3(0.6, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .into(),
        ..default()
    });

    // Camera
    let mut camera = commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(1.0, 0.7, 1.2)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        },
        ContrastAdaptiveSharpeningSettings {
            enabled: false,
            ..default()
        },
    ));
    if screenshot_taa.is_some() {
        camera.insert(TemporalAntiAliasBundle::default());
    }

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn update_uv_scale(
    main_window: Query<&Window, With<PrimaryWindow>>,
    noise_plane_mesh: Res<NoisePlaneMesh>,
    mut meshes: ResMut<Assets<Mesh>>,
    resize_events: EventReader<WindowResized>,
) {
    if resize_events.is_empty() {
        return;
    }
    let window_height = main_window.single().resolution.physical_height();
    let noise_mesh = meshes.get_mut(noise_plane_mesh.0.clone()).unwrap();
    set_noise_mesh_uv_scale(window_height, noise_mesh);
}

fn set_noise_mesh_uv_scale(window_height: u32, noise_mesh: &mut Mesh) {
    // modify the uvs so the texture repeats on the plane relative to the window height
    let uvscale = (window_height as f32 / 720.0) * 10.0;
    if let Some(VertexAttributeValues::Float32x2(uvs)) =
        noise_mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
    {
        for uv in uvs {
            if uv[0] > 0.0 {
                uv[0] = uvscale;
            }
            if uv[1] > 0.0 {
                uv[1] = uvscale;
            }
        }
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    img.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor::default());
    img
}

pub fn uhash(a: u32, b: u32) -> u32 {
    let mut x = (a.overflowing_mul(1597334673u32).0) ^ (b.overflowing_mul(3812015801u32).0);
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u32);
    x = x.overflowing_mul(0x7feb352du32).0;
    x = x ^ (x >> 15u32);
    x = x.overflowing_mul(0x846ca68bu32).0;
    x = x ^ (x >> 16u32);
    x
}

pub fn unormf(n: u32) -> f32 {
    n as f32 * (1.0 / 0xffffffffu32 as f32)
}

pub fn hash_noise(ufrag_coord: UVec2, frame: u32) -> f32 {
    let urnd = uhash(ufrag_coord.x, (ufrag_coord.y << 11u32) + frame);
    unormf(urnd)
}

/// Creates a noise texture
fn noise_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 256;

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for (i, val) in texture_data.iter_mut().enumerate() {
        if i % 4 == 3 {
            // Make all pixels opaque
            *val = 255;
        } else {
            let x = (i % (TEXTURE_SIZE * 4)) as u32;
            let y = (i / (TEXTURE_SIZE * 4)) as u32;
            let urand = hash_noise(uvec2(x, y), 0);
            *val = (urand.powf(2.2) * 255.0 + 0.5) as u8;
        }
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Generate mips
    let mut dyn_image = img.clone().try_into_dynamic().unwrap();
    let mut image_data = dyn_image.as_bytes().to_vec();
    let mut size = TEXTURE_SIZE as u32;
    let mut mip_count = 1;
    while size / 2 >= 2 {
        size /= 2;
        dyn_image = dyn_image.resize_exact(size, size, FilterType::Triangle);
        image_data.append(&mut dyn_image.as_bytes().to_vec());
        mip_count += 1;
    }

    img.data = image_data;
    img.texture_descriptor.mip_level_count = mip_count;
    img.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    img
}
