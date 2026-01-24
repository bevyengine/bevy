//! Demonstrates connecting color target input and output of multiple cameras.
//!

use bevy::image::ToExtents;
use bevy::render::{
    render_resource::{
        BlendState, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    view::Hdr,
};
use bevy::{
    camera::{
        color_target::{
            MainColorTarget, MainColorTargetInput, MainColorTargetInputConfig,
            NoAutoConfiguredMainColorTarget, WithMainColorTarget, MAIN_COLOR_TARGET_DEFAULT_USAGES,
        },
        visibility::RenderLayers,
        CameraOutputMode, RenderTarget,
    },
    core_pipeline::tonemapping::Tonemapping,
    post_process::{
        bloom::{Bloom, BloomCompositeMode},
        effect_stack::Vignette,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let transform = Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
    let uv_checker_image = asset_server.load::<Image>("textures/uv_checker_bw.png");

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.4, 0.6, 0.4))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::linear_rgb(5.0, 30.0, 1.0))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    let main_a = images.add(Image {
        data: None,
        texture_descriptor: TextureDescriptor {
            label: Some("manual_main_texture_a"),
            size: UVec2::new(1280, 720).to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg11b10Ufloat,
            usage: MAIN_COLOR_TARGET_DEFAULT_USAGES,
            view_formats: &[],
        },
        ..Default::default()
    });
    let main_b = images.add(Image {
        data: None,
        texture_descriptor: TextureDescriptor {
            label: Some("manual_main_texture_b"),
            size: UVec2::new(1280, 720).to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg11b10Ufloat,
            usage: MAIN_COLOR_TARGET_DEFAULT_USAGES,
            view_formats: &[],
        },
        ..Default::default()
    });
    let main_color_target = commands
        .spawn(MainColorTarget::new(
            main_a,
            Some(main_b.clone()),
            Some(images.add(Image {
                data: None,
                texture_descriptor: TextureDescriptor {
                    label: Some("manual_main_texture_multisampled"),
                    size: UVec2::new(1280, 720).to_extents(),
                    mip_level_count: 1,
                    sample_count: 4, // MSAAx4
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rg11b10Ufloat,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                ..Default::default()
            })),
        ))
        .id();

    // UI
    let layer2 = RenderLayers::layer(2);
    let ui_l2 = commands
        .spawn((
            Text::new(
                r#"
    pass0 --> pass1 --+
                      |
    pass3 --> pass2 --+--> pass4 --> screen

"#,
            ),
            TextColor(Color::srgba_u8(0x4b, 0xd0, 0x73, 255)),
            BackgroundColor(Color::srgba_u8(0x21, 0x4c, 0x76, 200)),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                ..default()
            },
            layer2.clone(),
        ))
        .id();
    let layer3 = RenderLayers::layer(3);
    let ui_l3 = commands
        .spawn((
            BackgroundColor(Color::srgba_u8(0x7a, 0x63, 0xae, 135)),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            layer3.clone(),
        ))
        .id();

    // Pass 0: reads from image and renders main textures, no output.
    let _pass0 = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                // Don't use clear color since the color target will be filled with image.
                clear_color: ClearColorConfig::None,
                // Skip upscaling to screen.
                output_mode: CameraOutputMode::Skip,
                ..Default::default()
            },
            // Enable Hdr to bypass in-shader tonemapping.
            Hdr,
            // Bypass tonemapping.
            Tonemapping::None,
            NoAutoConfiguredMainColorTarget,
            WithMainColorTarget(main_color_target),
            transform,
        ))
        .with_related::<MainColorTargetInput>(MainColorTarget::new(uv_checker_image, None, None))
        .id();

    // Pass 1: renders main textures with effects.
    let layer1 = RenderLayers::layer(1);
    let _pass1 = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 1,
                // Don't use clear color since the color target is filled in pass 0.
                clear_color: ClearColorConfig::None,
                // Skip upscaling to screen.
                output_mode: CameraOutputMode::Skip,
                ..Default::default()
            },
            Hdr,
            Bloom {
                composite_mode: BloomCompositeMode::Additive,
                intensity: 0.1,
                ..Bloom::NATURAL
            },
            Tonemapping::TonyMcMapface,
            NoAutoConfiguredMainColorTarget,
            WithMainColorTarget(main_color_target),
            layer1,
        ))
        .id();

    // Pass 2: renders UI.
    let _pass2 = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 2,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                // Skip upscaling to screen.
                output_mode: CameraOutputMode::Skip,
                ..Default::default()
            },
            MainColorTargetInputConfig {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                order: 1,
            },
            layer2,
        ))
        .id();
    commands.entity(ui_l2).insert(UiTargetCamera(_pass2));

    // Pass 3: renders UI.
    let _pass3 = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 3,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                output_mode: CameraOutputMode::Write {
                    blend_state: Some(BlendState::ALPHA_BLENDING),
                    clear_color: ClearColorConfig::None,
                },
                ..Default::default()
            },
            RenderTarget::MainColorTarget(_pass2),
            layer3,
        ))
        .id();
    commands.entity(ui_l3).insert(UiTargetCamera(_pass3));

    // Pass 4: renders to screen.
    let layer4 = RenderLayers::layer(4);
    let _pass4 = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 4,
                // Don't use clear color since the color target is filled with `main_color_target`.
                clear_color: ClearColorConfig::None,
                ..Default::default()
            },
            Vignette {
                radius: 1.0,
                ..Default::default()
            },
            layer4,
        ))
        .add_related::<MainColorTargetInput>(&[main_color_target, _pass2])
        .id();
}
