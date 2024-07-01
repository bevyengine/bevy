//! This example compares MSAA (Multi-Sample Anti-aliasing), FXAA (Fast Approximate Anti-aliasing), and TAA (Temporal Anti-aliasing).

use std::f32::consts::PI;
use std::fmt::Write;

use bevy::{
    core_pipeline::{
        contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings,
        experimental::taa::{
            TemporalAntiAliasBundle, TemporalAntiAliasPlugin, TemporalAntiAliasSettings,
        },
        fxaa::{Fxaa, Sensitivity},
        smaa::{SmaaPreset, SmaaSettings},
    },
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{ImageSampler, ImageSamplerDescriptor},
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
    keys: Res<ButtonInput<KeyCode>>,
    mut camera: Query<
        (
            Entity,
            Option<&mut Fxaa>,
            Option<&mut SmaaSettings>,
            Option<&TemporalAntiAliasSettings>,
        ),
        With<Camera>,
    >,
    mut msaa: ResMut<Msaa>,
    mut commands: Commands,
) {
    let (camera_entity, fxaa, smaa, taa) = camera.single_mut();
    let mut camera = commands.entity(camera_entity);

    // No AA
    if keys.just_pressed(KeyCode::Digit1) {
        *msaa = Msaa::Off;
        camera.remove::<Fxaa>();
        camera.remove::<SmaaSettings>();
        camera.remove::<TemporalAntiAliasBundle>();
    }

    // MSAA
    if keys.just_pressed(KeyCode::Digit2) && *msaa == Msaa::Off {
        camera.remove::<Fxaa>();
        camera.remove::<SmaaSettings>();
        camera.remove::<TemporalAntiAliasBundle>();

        *msaa = Msaa::Sample4;
    }

    // MSAA Sample Count
    if *msaa != Msaa::Off {
        if keys.just_pressed(KeyCode::KeyQ) {
            *msaa = Msaa::Sample2;
        }
        if keys.just_pressed(KeyCode::KeyW) {
            *msaa = Msaa::Sample4;
        }
        if keys.just_pressed(KeyCode::KeyE) {
            *msaa = Msaa::Sample8;
        }
    }

    // FXAA
    if keys.just_pressed(KeyCode::Digit3) && fxaa.is_none() {
        *msaa = Msaa::Off;
        camera.remove::<SmaaSettings>();
        camera.remove::<TemporalAntiAliasBundle>();

        camera.insert(Fxaa::default());
    }

    // FXAA Settings
    if let Some(mut fxaa) = fxaa {
        if keys.just_pressed(KeyCode::KeyQ) {
            fxaa.edge_threshold = Sensitivity::Low;
            fxaa.edge_threshold_min = Sensitivity::Low;
        }
        if keys.just_pressed(KeyCode::KeyW) {
            fxaa.edge_threshold = Sensitivity::Medium;
            fxaa.edge_threshold_min = Sensitivity::Medium;
        }
        if keys.just_pressed(KeyCode::KeyE) {
            fxaa.edge_threshold = Sensitivity::High;
            fxaa.edge_threshold_min = Sensitivity::High;
        }
        if keys.just_pressed(KeyCode::KeyR) {
            fxaa.edge_threshold = Sensitivity::Ultra;
            fxaa.edge_threshold_min = Sensitivity::Ultra;
        }
        if keys.just_pressed(KeyCode::KeyT) {
            fxaa.edge_threshold = Sensitivity::Extreme;
            fxaa.edge_threshold_min = Sensitivity::Extreme;
        }
    }

    // SMAA
    if keys.just_pressed(KeyCode::Digit4) && smaa.is_none() {
        *msaa = Msaa::Off;
        camera.remove::<Fxaa>();
        camera.remove::<TemporalAntiAliasBundle>();

        camera.insert(SmaaSettings::default());
    }

    // SMAA Settings
    if let Some(mut smaa) = smaa {
        if keys.just_pressed(KeyCode::KeyQ) {
            smaa.preset = SmaaPreset::Low;
        }
        if keys.just_pressed(KeyCode::KeyW) {
            smaa.preset = SmaaPreset::Medium;
        }
        if keys.just_pressed(KeyCode::KeyE) {
            smaa.preset = SmaaPreset::High;
        }
        if keys.just_pressed(KeyCode::KeyR) {
            smaa.preset = SmaaPreset::Ultra;
        }
    }

    // TAA
    if keys.just_pressed(KeyCode::Digit5) && taa.is_none() {
        *msaa = Msaa::Off;
        camera.remove::<Fxaa>();
        camera.remove::<SmaaSettings>();

        camera.insert(TemporalAntiAliasBundle::default());
    }
}

fn modify_sharpening(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut ContrastAdaptiveSharpeningSettings>,
) {
    for mut cas in &mut query {
        if keys.just_pressed(KeyCode::Digit0) {
            cas.enabled = !cas.enabled;
        }
        if cas.enabled {
            if keys.just_pressed(KeyCode::Minus) {
                cas.sharpening_strength -= 0.1;
                cas.sharpening_strength = cas.sharpening_strength.clamp(0.0, 1.0);
            }
            if keys.just_pressed(KeyCode::Equal) {
                cas.sharpening_strength += 0.1;
                cas.sharpening_strength = cas.sharpening_strength.clamp(0.0, 1.0);
            }
            if keys.just_pressed(KeyCode::KeyD) {
                cas.denoise = !cas.denoise;
            }
        }
    }
}

fn update_ui(
    camera: Query<
        (
            Option<&Fxaa>,
            Option<&SmaaSettings>,
            Option<&TemporalAntiAliasSettings>,
            &ContrastAdaptiveSharpeningSettings,
        ),
        With<Camera>,
    >,
    msaa: Res<Msaa>,
    mut ui: Query<&mut Text>,
) {
    let (fxaa, smaa, taa, cas_settings) = camera.single();

    let mut ui = ui.single_mut();
    let ui = &mut ui.sections[0].value;

    *ui = "Antialias Method\n".to_string();

    draw_selectable_menu_item(
        ui,
        "No AA",
        '1',
        *msaa == Msaa::Off && fxaa.is_none() && taa.is_none() && smaa.is_none(),
    );
    draw_selectable_menu_item(ui, "MSAA", '2', *msaa != Msaa::Off);
    draw_selectable_menu_item(ui, "FXAA", '3', fxaa.is_some());
    draw_selectable_menu_item(ui, "SMAA", '4', smaa.is_some());
    draw_selectable_menu_item(ui, "TAA", '5', taa.is_some());

    if *msaa != Msaa::Off {
        ui.push_str("\n----------\n\nSample Count\n");
        draw_selectable_menu_item(ui, "2", 'Q', *msaa == Msaa::Sample2);
        draw_selectable_menu_item(ui, "4", 'W', *msaa == Msaa::Sample4);
        draw_selectable_menu_item(ui, "8", 'E', *msaa == Msaa::Sample8);
    }

    if let Some(fxaa) = fxaa {
        ui.push_str("\n----------\n\nSensitivity\n");
        draw_selectable_menu_item(ui, "Low", 'Q', fxaa.edge_threshold == Sensitivity::Low);
        draw_selectable_menu_item(
            ui,
            "Medium",
            'W',
            fxaa.edge_threshold == Sensitivity::Medium,
        );
        draw_selectable_menu_item(ui, "High", 'E', fxaa.edge_threshold == Sensitivity::High);
        draw_selectable_menu_item(ui, "Ultra", 'R', fxaa.edge_threshold == Sensitivity::Ultra);
        draw_selectable_menu_item(
            ui,
            "Extreme",
            'T',
            fxaa.edge_threshold == Sensitivity::Extreme,
        );
    }

    if let Some(smaa) = smaa {
        ui.push_str("\n----------\n\nQuality\n");
        draw_selectable_menu_item(ui, "Low", 'Q', smaa.preset == SmaaPreset::Low);
        draw_selectable_menu_item(ui, "Medium", 'W', smaa.preset == SmaaPreset::Medium);
        draw_selectable_menu_item(ui, "High", 'E', smaa.preset == SmaaPreset::High);
        draw_selectable_menu_item(ui, "Ultra", 'R', smaa.preset == SmaaPreset::Ultra);
    }

    ui.push_str("\n----------\n\n");
    draw_selectable_menu_item(ui, "Sharpening", '0', cas_settings.enabled);

    if cas_settings.enabled {
        ui.push_str(&format!(
            "(-/+) Strength: {:.1}\n",
            cas_settings.sharpening_strength
        ));
        draw_selectable_menu_item(ui, "Denoising", 'D', cas_settings.denoise);
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
        mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
        material: materials.add(Color::srgb(0.1, 0.2, 0.1)),
        ..default()
    });

    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Cubes
    for i in 0..5 {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.25, 0.25, 0.25)),
            material: cube_material.clone(),
            transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
            ..default()
        });
    }

    // Flight Helmet
    commands.spawn(SceneBundle {
        scene: asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
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
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
        },
        FogSettings {
            color: Color::srgba_u8(43, 44, 47, 255),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 4.0,
            },
            ..default()
        },
    ));

    // example instructions
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

/// Writes a simple menu item that can be on or off.
fn draw_selectable_menu_item(ui: &mut String, label: &str, shortcut: char, enabled: bool) {
    let star = if enabled { "*" } else { "" };
    let _ = writeln!(*ui, "({}) {}{}{}", shortcut, star, label, star);
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
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::default());
    img
}
