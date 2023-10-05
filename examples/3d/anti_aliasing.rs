//! This example compares MSAA (Multi-Sample Anti-aliasing), FXAA (Fast Approximate Anti-aliasing), and TAA (Temporal Anti-aliasing).

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use std::f32::consts::PI;

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

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((DefaultPlugins, TemporalAntiAliasPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (modify_aa, modify_sharpening, update_ui))
        .run();
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
    camera: Query<
        (
            Option<&Fxaa>,
            Option<&TemporalAntiAliasSettings>,
            &ContrastAdaptiveSharpeningSettings,
        ),
        With<Camera>,
    >,
    msaa: Res<Msaa>,
    mut ui: Query<&mut Text>,
) {
    let (fxaa, taa, cas_settings) = camera.single();

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
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Cubes
    for i in 0..5 {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.25 })),
            material: cube_material.clone(),
            transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
            ..default()
        });
    }

    // Flight Helmet
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
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
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        },
        ContrastAdaptiveSharpeningSettings {
            enabled: false,
            ..default()
        },
    ));

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
