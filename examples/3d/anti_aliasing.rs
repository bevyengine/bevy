//! Compares different anti-aliasing techniques supported by Bevy.

use std::{f32::consts::PI, fmt::Write};

use bevy::{
    anti_alias::{
        contrast_adaptive_sharpening::ContrastAdaptiveSharpening,
        fxaa::{Fxaa, Sensitivity},
        smaa::{Smaa, SmaaPreset},
        taa::TemporalAntiAliasing,
    },
    asset::RenderAssetUsages,
    core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass},
    image::{ImageSampler, ImageSamplerDescriptor},
    light::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        camera::{MipBias, TemporalJitter},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::Hdr,
    },
};

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy::anti_alias::dlss::{
    Dlss, DlssPerfQualityMode, DlssProjectId, DlssSuperResolutionSupported,
};

fn main() {
    let mut app = App::new();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy_asset::uuid::uuid!(
        "5417916c-0291-4e3f-8f65-326c1858ab96" // Don't copy paste this - generate your own UUID!
    )));

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (modify_aa, modify_sharpening, modify_projection, update_ui),
        );

    app.run();
}

type TaaComponents = (
    TemporalAntiAliasing,
    TemporalJitter,
    MipBias,
    DepthPrepass,
    MotionVectorPrepass,
);

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
type DlssComponents = (
    Dlss,
    TemporalJitter,
    MipBias,
    DepthPrepass,
    MotionVectorPrepass,
);
#[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
type DlssComponents = ();

fn modify_aa(
    keys: Res<ButtonInput<KeyCode>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] camera: Single<
        (
            Entity,
            Option<&mut Fxaa>,
            Option<&mut Smaa>,
            Option<&TemporalAntiAliasing>,
            &mut Msaa,
            Option<&mut Dlss>,
        ),
        With<Camera>,
    >,
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))] camera: Single<
        (
            Entity,
            Option<&mut Fxaa>,
            Option<&mut Smaa>,
            Option<&TemporalAntiAliasing>,
            &mut Msaa,
        ),
        With<Camera>,
    >,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_supported: Option<
        Res<DlssSuperResolutionSupported>,
    >,
    mut commands: Commands,
) {
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let (camera_entity, fxaa, smaa, taa, mut msaa, dlss) = camera.into_inner();
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let (camera_entity, fxaa, smaa, taa, mut msaa) = camera.into_inner();
    let mut camera = commands.entity(camera_entity);

    // No AA
    if keys.just_pressed(KeyCode::Digit1) {
        *msaa = Msaa::Off;
        camera
            .remove::<Fxaa>()
            .remove::<Smaa>()
            .remove::<TaaComponents>()
            .remove::<DlssComponents>();
    }

    // MSAA
    if keys.just_pressed(KeyCode::Digit2) && *msaa == Msaa::Off {
        camera
            .remove::<Fxaa>()
            .remove::<Smaa>()
            .remove::<TaaComponents>()
            .remove::<DlssComponents>();

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
        camera
            .remove::<Smaa>()
            .remove::<TaaComponents>()
            .remove::<DlssComponents>()
            .insert(Fxaa::default());
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
        camera
            .remove::<Fxaa>()
            .remove::<TaaComponents>()
            .remove::<DlssComponents>()
            .insert(Smaa::default());
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
        camera
            .remove::<Fxaa>()
            .remove::<Smaa>()
            .remove::<DlssComponents>()
            .insert(TemporalAntiAliasing::default());
    }

    // DLSS
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if keys.just_pressed(KeyCode::Digit6) && dlss.is_none() && dlss_supported.is_some() {
        *msaa = Msaa::Off;
        camera
            .remove::<Fxaa>()
            .remove::<Smaa>()
            .remove::<TaaComponents>()
            .insert(Dlss::default());
    }

    // DLSS Settings
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(mut dlss) = dlss {
        if keys.just_pressed(KeyCode::KeyZ) {
            dlss.perf_quality_mode = DlssPerfQualityMode::Auto;
        }
        if keys.just_pressed(KeyCode::KeyX) {
            dlss.perf_quality_mode = DlssPerfQualityMode::UltraPerformance;
        }
        if keys.just_pressed(KeyCode::KeyC) {
            dlss.perf_quality_mode = DlssPerfQualityMode::Performance;
        }
        if keys.just_pressed(KeyCode::KeyV) {
            dlss.perf_quality_mode = DlssPerfQualityMode::Balanced;
        }
        if keys.just_pressed(KeyCode::KeyB) {
            dlss.perf_quality_mode = DlssPerfQualityMode::Quality;
        }
        if keys.just_pressed(KeyCode::KeyN) {
            dlss.perf_quality_mode = DlssPerfQualityMode::Dlaa;
        }
    }
}

fn modify_sharpening(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut ContrastAdaptiveSharpening>,
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

fn modify_projection(keys: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Projection>) {
    for mut projection in &mut query {
        if keys.just_pressed(KeyCode::KeyO) {
            match *projection {
                Projection::Perspective(_) => {
                    *projection = Projection::Orthographic(OrthographicProjection {
                        scale: 0.002,
                        ..OrthographicProjection::default_3d()
                    });
                }
                _ => {
                    *projection = Projection::Perspective(PerspectiveProjection::default());
                }
            }
        }
    }
}

fn update_ui(
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] camera: Single<
        (
            &Projection,
            Option<&Fxaa>,
            Option<&Smaa>,
            Option<&TemporalAntiAliasing>,
            &ContrastAdaptiveSharpening,
            &Msaa,
            Option<&Dlss>,
        ),
        With<Camera>,
    >,
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))] camera: Single<
        (
            &Projection,
            Option<&Fxaa>,
            Option<&Smaa>,
            Option<&TemporalAntiAliasing>,
            &ContrastAdaptiveSharpening,
            &Msaa,
        ),
        With<Camera>,
    >,
    mut ui: Single<&mut Text>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_supported: Option<
        Res<DlssSuperResolutionSupported>,
    >,
) {
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let (projection, fxaa, smaa, taa, cas, msaa, dlss) = *camera;
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let (projection, fxaa, smaa, taa, cas, msaa) = *camera;

    let ui = &mut ui.0;
    *ui = "Antialias Method\n".to_string();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let dlss_none = dlss.is_none();
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let dlss_none = true;

    draw_selectable_menu_item(
        ui,
        "No AA",
        '1',
        *msaa == Msaa::Off && fxaa.is_none() && taa.is_none() && smaa.is_none() && dlss_none,
    );
    draw_selectable_menu_item(ui, "MSAA", '2', *msaa != Msaa::Off);
    draw_selectable_menu_item(ui, "FXAA", '3', fxaa.is_some());
    draw_selectable_menu_item(ui, "SMAA", '4', smaa.is_some());
    draw_selectable_menu_item(ui, "TAA", '5', taa.is_some());
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_supported.is_some() {
        draw_selectable_menu_item(ui, "DLSS", '6', dlss.is_some());
    }

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

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(dlss) = dlss {
        let pqm = dlss.perf_quality_mode;
        ui.push_str("\n----------\n\nQuality\n");
        draw_selectable_menu_item(ui, "Auto", 'Z', pqm == DlssPerfQualityMode::Auto);
        draw_selectable_menu_item(
            ui,
            "UltraPerformance",
            'X',
            pqm == DlssPerfQualityMode::UltraPerformance,
        );
        draw_selectable_menu_item(
            ui,
            "Performance",
            'C',
            pqm == DlssPerfQualityMode::Performance,
        );
        draw_selectable_menu_item(ui, "Balanced", 'V', pqm == DlssPerfQualityMode::Balanced);
        draw_selectable_menu_item(ui, "Quality", 'B', pqm == DlssPerfQualityMode::Quality);
        draw_selectable_menu_item(ui, "DLAA", 'N', pqm == DlssPerfQualityMode::Dlaa);
    }

    ui.push_str("\n----------\n\n");
    draw_selectable_menu_item(ui, "Sharpening", '0', cas.enabled);

    if cas.enabled {
        ui.push_str(&format!("(-/+) Strength: {:.1}\n", cas.sharpening_strength));
        draw_selectable_menu_item(ui, "Denoising", 'D', cas.denoise);
    }

    ui.push_str("\n----------\n\n");
    draw_selectable_menu_item(
        ui,
        "Orthographic",
        'O',
        matches!(projection, Projection::Orthographic(_)),
    );
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
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.2, 0.1))),
    ));

    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // Cubes
    for i in 0..5 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.25, 0.25, 0.25))),
            MeshMaterial3d(cube_material.clone()),
            Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
        ));
    }

    // Flight Helmet
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
    )));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
        CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .build(),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ContrastAdaptiveSharpening {
            enabled: false,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
            ..default()
        },
        DistanceFog {
            color: Color::srgba_u8(43, 44, 47, 255),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 4.0,
            },
            ..default()
        },
    ));

    // example instructions
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Writes a simple menu item that can be on or off.
fn draw_selectable_menu_item(ui: &mut String, label: &str, shortcut: char, enabled: bool) {
    let star = if enabled { "*" } else { "" };
    let _ = writeln!(*ui, "({shortcut}) {star}{label}{star}");
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
