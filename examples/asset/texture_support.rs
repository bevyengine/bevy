//! This example demonstrates a selection of supported texture formats.
//!
//! ## Controls
//!
//! | Key Binding          | Action        |
//! |:---------------------|:--------------|
//! | `Arrow Up`           | Move up       |
//! | `Arrow Down`         | Move down     |
//! | `Arrow Left`         | Move left     |
//! | `Arrow Right`        | Move right    |
//! | `+`                  | Zoom in       |
//! | `-`                  | Zoom out      |

use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
        tonemapping::Tonemapping,
    },
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    sprite::{AlphaMode2d, Material2d, Material2dPlugin},
    text::TextBounds,
};
use bevy_image::{ImageLoaderSettings, ImageSampler};
use bevy_render::{render_asset::RenderAssets, render_resource::*, texture::GpuImage};

const MIN_CAMERA_SCALE: f32 = 0.05;
const MAX_CAMERA_SCALE: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextureSupportMaterialsPlugin::default())
        .add_systems(Startup, (setup_scene_textures, setup_camera))
        .add_systems(
            Update,
            (handle_mouse_drag, handle_mouse_scroll, handle_keyboard).chain(),
        )
        .insert_resource(ClearColor::default())
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true, // HDR is required for the bloom effect
            ..default()
        },
        Tonemapping::None,
        // In this example we visualize HDR using bloom.
        Bloom {
            composite_mode: BloomCompositeMode::Additive,
            intensity: 0.6,
            high_pass_frequency: 1.0,
            low_frequency_boost: 0.2,
            low_frequency_boost_curvature: 0.95,
            // Only apply bloom to texture values > 1.0.
            prefilter: BloomPrefilter {
                threshold: 1.0,
                threshold_softness: 0.0,
            },
            ..Bloom::NATURAL
        },
        Transform::from_translation(Vec3::new(1061.7567, -581.69104, 0.0)),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.841956,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Information about the texture laying out in the scene.
struct TextureCell {
    mip_count: u32,
    layer_count: u32,
}

impl Default for TextureCell {
    fn default() -> Self {
        Self::single()
    }
}

impl TextureCell {
    // A single texture (no layers, no mips)
    fn single() -> Self {
        Self {
            mip_count: 1,
            layer_count: 1,
        }
    }
    // Sets the number of mip levels.
    fn with_mips(mut self, mip_count: u32) -> Self {
        self.mip_count = mip_count;
        self
    }
    // Sets the number of layers.
    fn with_layers(mut self, layer_count: u32) -> Self {
        self.layer_count = layer_count;
        self
    }
}

fn setup_scene_textures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut materials_2d: ResMut<Assets<Texture2dMipMaterial>>,
    mut materials_2d_array: ResMut<Assets<Texture2dArrayMipMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let uncompressed_formats = vec![
        (
            "PNG (sRGB)",
            "textures/texture_support/png-srgb-rgb.png",
            TextureCell::single(),
        ),
        (
            "KTX2 R32_SFLOAT",
            "textures/texture_support/ktx2-hdr-r32.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 R32G32B32_SFLOAT",
            "textures/texture_support/ktx2-hdr-rgb32.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 R32G32B32A32_SFLOAT",
            "textures/texture_support/ktx2-hdr-rgba32.ktx2",
            TextureCell::single(),
        ),
        (
            "EXR (HDR)",
            "textures/texture_support/exr-hdr.exr",
            TextureCell::single(),
        ),
        (
            "DDS",
            "textures/texture_support/dds.dds",
            TextureCell::single(),
        ),
        (
            "BMP",
            "textures/texture_support/bmp.bmp",
            TextureCell::single(),
        ),
        (
            "GIF",
            "textures/texture_support/gif.gif",
            TextureCell::single(),
        ),
        (
            "ICO",
            "textures/texture_support/ico.ico",
            TextureCell::single(),
        ),
        (
            "JPEG",
            "textures/texture_support/jpg.jpg",
            TextureCell::single(),
        ),
        (
            "PPM",
            "textures/texture_support/ppm.ppm",
            TextureCell::single(),
        ),
        (
            "PAM",
            "textures/texture_support/pam.pam",
            TextureCell::single(),
        ),
        (
            "QOI",
            "textures/texture_support/qoi.qoi",
            TextureCell::single(),
        ),
        (
            "Tiff",
            "textures/texture_support/tif.tif",
            TextureCell::single(),
        ),
        (
            "TGA",
            "textures/texture_support/tga.tga",
            TextureCell::single(),
        ),
        (
            "WebP",
            "textures/texture_support/webp.webp",
            TextureCell::single(),
        ),
    ];

    let ktx2_astc_formats = vec![
        (
            "KTX2 ASTC 4x4 HDR",
            "textures/texture_support/ktx2-astc-4x4-hdr.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x6 HDR",
            "textures/texture_support/ktx2-astc-6x6-hdr.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x6 Intermediate HDR",
            "textures/texture_support/ktx2-astc-6x6i-hdr.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 Zstd ASTC 4x4 sRGB",
            "textures/texture_support/ktx2-zstd-astc-4x4-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 Zstd ASTC 4x4 Linear",
            "textures/texture_support/ktx2-zstd-astc-4x4-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 4x4 sRGB",
            "textures/texture_support/ktx2-astc-4x4-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 4x4 Linear",
            "textures/texture_support/ktx2-astc-4x4-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 5x4 sRGB",
            "textures/texture_support/ktx2-astc-5x4-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 5x4 Linear",
            "textures/texture_support/ktx2-astc-5x4-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 5x5 sRGB",
            "textures/texture_support/ktx2-astc-5x5-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 5x5 Linear",
            "textures/texture_support/ktx2-astc-5x5-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x5 sRGB",
            "textures/texture_support/ktx2-astc-6x5-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x5 Linear",
            "textures/texture_support/ktx2-astc-6x5-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x6 sRGB",
            "textures/texture_support/ktx2-astc-6x6-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 6x6 Linear",
            "textures/texture_support/ktx2-astc-6x6-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x5 sRGB",
            "textures/texture_support/ktx2-astc-8x5-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x5 Linear",
            "textures/texture_support/ktx2-astc-8x5-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x6 sRGB",
            "textures/texture_support/ktx2-astc-8x6-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x6 Linear",
            "textures/texture_support/ktx2-astc-8x6-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x5 sRGB",
            "textures/texture_support/ktx2-astc-10x5-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x5 Linear",
            "textures/texture_support/ktx2-astc-10x5-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x6 sRGB",
            "textures/texture_support/ktx2-astc-10x6-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x6 Linear",
            "textures/texture_support/ktx2-astc-10x6-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x8 sRGB",
            "textures/texture_support/ktx2-astc-8x8-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 8x8 Linear",
            "textures/texture_support/ktx2-astc-8x8-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x8 sRGB",
            "textures/texture_support/ktx2-astc-10x8-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x8 Linear",
            "textures/texture_support/ktx2-astc-10x8-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x10 sRGB",
            "textures/texture_support/ktx2-astc-10x10-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 10x10 Linear",
            "textures/texture_support/ktx2-astc-10x10-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 12x10 sRGB",
            "textures/texture_support/ktx2-astc-12x10-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 12x10 Linear",
            "textures/texture_support/ktx2-astc-12x10-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 12x12 sRGB",
            "textures/texture_support/ktx2-astc-12x12-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ASTC 12x12 Linear",
            "textures/texture_support/ktx2-astc-12x12-linear.ktx2",
            TextureCell::single(),
        ),
    ];

    let ktx2_uastc_formats = vec![
        (
            "KTX2 UASTC sRGB",
            "textures/texture_support/ktx2-uastc-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 UASTC sRGB w/RDO",
            "textures/texture_support/ktx2-uastc-srgb-rdo.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 UASTC Linear",
            "textures/texture_support/ktx2-uastc-linear.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 UASTC Linear w/RDO",
            "textures/texture_support/ktx2-uastc-linear-rdo.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 Zstd UASTC sRGB w/RDO",
            "textures/texture_support/ktx2-zstd-uastc-srgb-rdo.ktx2",
            TextureCell::single(),
        ),
    ];

    let ktx2_etc1s_formats = vec![
        (
            "KTX2 ETC1S/BasisLZ sRGB",
            "textures/texture_support/ktx2-etc1s-srgb.ktx2",
            TextureCell::single(),
        ),
        (
            "KTX2 ETC1S/BasisLZ Linear",
            "textures/texture_support/ktx2-etc1s-linear.ktx2",
            TextureCell::single(),
        ),
    ];

    let basis_formats = vec![
        (
            "Basis ETC1S sRGB",
            "textures/texture_support/basis-etc1s-srgb.basis",
            TextureCell::single(),
        ),
        (
            "Basis ETC1S Linear",
            "textures/texture_support/basis-etc1s-linear.basis",
            TextureCell::single(),
        ),
        (
            "Basis UASTC 4x4 Linear",
            "textures/texture_support/basis-uastc-4x4-srgb.basis",
            TextureCell::single(),
        ),
        (
            "Basis UASTC 4x4 w/RDO Linear",
            "textures/texture_support/basis-uastc-4x4-rdo-srgb.basis",
            TextureCell::single(),
        ),
        (
            "Basis ASTC 4x4 HDR",
            "textures/texture_support/basis-astc-4x4-hdr.basis",
            TextureCell::single(),
        ),
        (
            "Basis ASTC 6x6 HDR",
            "textures/texture_support/basis-astc-6x6-hdr.basis",
            TextureCell::single(),
        ),
        (
            "Basis ASTC 6x6 Intermediate HDR",
            "textures/texture_support/basis-astc-6x6i-hdr.basis",
            TextureCell::single(),
        ),
    ];

    let misc_formats = vec![
        (
            "KTX2 w/Layers & Mips",
            "textures/texture_support/ktx2-astc-4x4-srgb-multilayer-mips.ktx2",
            TextureCell::default().with_layers(6).with_mips(8),
        ),
        (
            "Basis w/Mips",
            "textures/texture_support/basis-uastc-4x4-srgb-mips.basis",
            TextureCell::default().with_layers(1).with_mips(8),
        ),
        (
            "Basis UASTC w/Layers & Mips",
            "textures/texture_support/basis-uastc-4x4-srgb-multilayer-mips.basis",
            TextureCell::default().with_layers(6).with_mips(8),
        ),
    ];

    #[derive(PartialEq)]
    enum FormatGroup {
        Uncompressed,
        Ktx2Astc,
        Ktx2Uastc,
        Ktx2Etc1s,
        Basis,
        Misc,
    }

    impl FormatGroup {
        fn label(&self) -> &str {
            match self {
                Self::Uncompressed => "Uncompressed",
                Self::Ktx2Astc => "KTX2 ASTC",
                Self::Ktx2Uastc => "KTX2 UASTC",
                Self::Ktx2Etc1s => "KTX2 ETC1S/BasisLZ",
                Self::Basis => "Basis Universal (.basis)",
                Self::Misc => "Misc",
            }
        }
    }

    let format_groups = [
        (FormatGroup::Uncompressed, uncompressed_formats),
        (FormatGroup::Ktx2Astc, ktx2_astc_formats),
        (FormatGroup::Ktx2Uastc, ktx2_uastc_formats),
        (FormatGroup::Ktx2Etc1s, ktx2_etc1s_formats),
        (FormatGroup::Basis, basis_formats),
        (FormatGroup::Misc, misc_formats),
    ];

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 14.0,
        ..default()
    };
    let header_font = TextFont {
        font: font.clone(),
        font_size: 24.0,
        ..default()
    };

    let col_spacing = 10.0;
    let row_spacing = 10.0;
    let row_wrap_width = 2000.0;
    let group_spacing = row_spacing * 2.0;
    let label_margin = row_spacing;
    let cell_padding = 5.0;
    let mip_spacing = 2.0;
    let layer_spacing = mip_spacing;

    let (image_width, image_height) = (200.0, 200.0);
    let (half_width, half_height) = (image_width / 2.0, image_height / 2.0);
    let text_box_size = Vec2::new(image_width, text_font.font_size * 1.2);

    let group_header_box_size = Vec2::new(image_width * 2.0, header_font.font_size * 1.2);
    let group_header_margin = label_margin;

    let cell_background_z = 0.8;
    let cell_content_z = 1.0;
    let cell_background_material = color_materials.add(ColorMaterial {
        color: Color::BLACK,
        ..Default::default()
    });

    let unit_quad = meshes.add(Rectangle::new(1.0, 1.0));

    let group_x = 0.0;
    let mut group_y = 0.0;

    // Main Header
    let main_header_font = TextFont {
        font: font.clone(),
        font_size: 32.0,
        ..default()
    };
    let main_header_box_size = Vec2::new(image_width * 2.0, main_header_font.font_size * 1.2);
    let main_header_margin = label_margin;
    commands.spawn((
        Text2d::new("Image / Texture Formats".to_string()),
        main_header_font.clone(),
        Transform::from_translation(Vec3::new(
            group_x + main_header_box_size.x * 0.5,
            -(group_y + main_header_box_size.y * 0.5),
            cell_content_z,
        )),
        TextLayout::new_with_justify(JustifyText::Left),
        TextBounds::from(main_header_box_size),
    ));
    group_y += main_header_box_size.y + main_header_margin;

    // Instructions
    let instructions_font = TextFont {
        font: font.clone(),
        font_size: 16.0,
        ..default()
    };
    let instructions_box_size = Vec2::new(image_width * 3.0, instructions_font.font_size * 1.2);
    let instructions_margin = label_margin * 2.0;
    commands.spawn((
        Text2d::new(
            "Move the camera with the arrow keys or mouse. Zoom with the +/- keys.".to_string(),
        ),
        instructions_font.clone(),
        Transform::from_translation(Vec3::new(
            group_x + instructions_box_size.x * 0.5,
            -(group_y + instructions_box_size.y * 0.5),
            cell_content_z,
        )),
        TextLayout::new_with_justify(JustifyText::Left),
        TextBounds::from(instructions_box_size),
    ));
    group_y += instructions_box_size.y + instructions_margin;

    for (format_group, format_list) in format_groups.into_iter() {
        // Group Header
        commands.spawn((
            Text2d::new(format_group.label().to_string()),
            header_font.clone(),
            Transform::from_translation(Vec3::new(
                group_x + group_header_box_size.x * 0.5,
                -(group_y + group_header_box_size.y * 0.5),
                cell_content_z,
            )),
            TextLayout::new_with_justify(JustifyText::Left),
            TextBounds::from(group_header_box_size),
        ));

        let mut col_x = group_x;
        let mut row_y = group_y + group_header_box_size.y + group_header_margin;

        let mut row_height = 0.0;
        let mut group_height = group_header_box_size.y + group_header_margin;

        for (format_label, image_path, texture_cell) in format_list {
            if col_x > row_wrap_width {
                group_height += row_height + row_spacing;
                col_x = group_x;
                row_y += row_height + row_spacing;
                row_height = 0.0;
            }

            let cell_x = col_x;
            let cell_y = row_y;

            let image_handle = asset_server.load_with_settings(
                image_path,
                |settings: &mut ImageLoaderSettings| {
                    settings.sampler = ImageSampler::nearest();
                },
            );

            if texture_cell.layer_count == 1 && texture_cell.mip_count == 1 {
                let cell_width = cell_padding * 2.0 + image_width;
                let cell_height =
                    cell_padding * 2.0 + image_height + label_margin + text_box_size.y;

                row_height = row_height.max(cell_height);

                // Cell Background
                commands.spawn((
                    Mesh2d(unit_quad.clone()),
                    MeshMaterial2d(cell_background_material.clone()),
                    Transform {
                        translation: Vec3::new(
                            cell_x + cell_width * 0.5,
                            -(cell_y + cell_height * 0.5),
                            cell_background_z,
                        ),
                        scale: Vec3::new(cell_width, cell_height, 1.0),
                        ..Default::default()
                    },
                ));

                // Texture Swatch
                commands.spawn((
                    Mesh2d(unit_quad.clone()),
                    MeshMaterial2d(materials_2d.add(Texture2dMipMaterial {
                        texture: image_handle,
                        mip_level: 0,
                    })),
                    Transform {
                        translation: Vec3::new(
                            cell_x + cell_padding + half_width,
                            -(cell_y + cell_padding + half_height),
                            cell_content_z,
                        ),
                        scale: Vec3::new(image_width, image_height, 1.0),
                        ..Default::default()
                    },
                ));

                // Texture Label
                commands.spawn((
                    Text2d::new(format_label.to_string()),
                    text_font.clone(),
                    Transform::from_translation(Vec3::new(
                        cell_x + cell_padding + half_width,
                        -(cell_y
                            + cell_padding
                            + image_height
                            + label_margin
                            + text_box_size.y * 0.5),
                        cell_content_z,
                    )),
                    TextLayout::new_with_justify(JustifyText::Center),
                    TextBounds::from(text_box_size),
                ));

                col_x += cell_width + col_spacing;
            } else if texture_cell.layer_count > 1 || texture_cell.mip_count > 1 {
                let image_width = image_width * 0.5;
                let image_height = image_height * 0.5;
                let half_width = image_width * 0.5;
                let half_height = image_height * 0.5;

                // X direction: Layers
                // Y direction: Mips
                let cell_width = cell_padding * 2.0
                    + image_width * (texture_cell.layer_count as f32)
                    + (texture_cell.layer_count as f32 - 1.0) * layer_spacing;
                let cell_height = cell_padding * 2.0
                    + (image_height + label_margin + text_box_size.y)
                        * (texture_cell.mip_count as f32)
                    + (texture_cell.mip_count as f32 - 1.0) * mip_spacing
                    + label_margin
                    + text_box_size.y;

                row_height = row_height.max(cell_height);

                // Cell Background
                commands.spawn((
                    Mesh2d(unit_quad.clone()),
                    MeshMaterial2d(cell_background_material.clone()),
                    Transform {
                        translation: Vec3::new(
                            cell_x + cell_width * 0.5,
                            -(cell_y + cell_height * 0.5),
                            cell_background_z,
                        ),
                        scale: Vec3::new(cell_width, cell_height, 1.0),
                        ..Default::default()
                    },
                ));

                // Texture Label
                commands.spawn((
                    Text2d::new(format_label.to_string()),
                    text_font.clone(),
                    Transform::from_translation(Vec3::new(
                        cell_x + cell_width * 0.5,
                        -(cell_y + cell_padding + text_box_size.y * 0.5),
                        cell_content_z,
                    )),
                    TextLayout::new_with_justify(JustifyText::Center),
                    TextBounds::from(Vec2::new(cell_width - cell_padding * 2.0, text_box_size.y)),
                ));

                for layer_index in 0..texture_cell.layer_count {
                    for mip_index in 0..texture_cell.mip_count {
                        let x = cell_x
                            + cell_padding
                            + layer_index as f32 * (layer_spacing + image_width);
                        let y = cell_y
                            + cell_padding
                            + text_box_size.y
                            + label_margin
                            + mip_index as f32
                                * (mip_spacing + image_width + text_box_size.y + label_margin);

                        // Texture Swatch
                        let mesh2d = Mesh2d(unit_quad.clone());
                        let transform = Transform {
                            translation: Vec3::new(
                                x + half_width,
                                -(y + half_height),
                                cell_content_z,
                            ),
                            scale: Vec3::new(image_width, image_height, 1.0),
                            ..Default::default()
                        };

                        if texture_cell.layer_count > 1 {
                            commands.spawn((
                                mesh2d,
                                transform,
                                MeshMaterial2d(materials_2d_array.add(Texture2dArrayMipMaterial {
                                    texture: image_handle.clone(),
                                    mip_level: mip_index,
                                    layer_index,
                                })),
                            ));
                        } else {
                            commands.spawn((
                                mesh2d,
                                transform,
                                MeshMaterial2d(materials_2d.add(Texture2dMipMaterial {
                                    texture: image_handle.clone(),
                                    mip_level: mip_index,
                                })),
                            ));
                        }

                        // Layer + Mip Label
                        commands.spawn((
                            Text2d::new(format!("Layer {layer_index} Mip {mip_index}")),
                            text_font.clone(),
                            Transform::from_translation(Vec3::new(
                                x + image_width * 0.5,
                                -(y + image_height + label_margin + text_box_size.y * 0.5 - 5.0),
                                cell_content_z,
                            )),
                            TextLayout::new_with_justify(JustifyText::Center),
                            TextBounds::from(text_box_size),
                        ));
                    }
                }

                col_x += cell_width + col_spacing;
            }
        }

        group_height += row_height;

        // Set the position of the next group
        group_y += group_height + group_spacing;
    }
}

fn handle_mouse_drag(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_event: EventReader<MouseMotion>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_event.read() {
        delta += event.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    let (mut camera_transform, mut camera_projection) = camera.into_inner();
    let mut current_scale = 1.0;
    if let Projection::Orthographic(camera_projection) = camera_projection.as_mut() {
        current_scale = camera_projection.scale;
    }
    camera_transform.translation += (current_scale * delta * Vec2::new(-1.0, 1.0)).extend(0.0);
}

fn handle_mouse_scroll(
    mut mouse_wheel_event: EventReader<MouseWheel>,
    mut camera_projection: Single<&mut Projection, With<Camera2d>>,
) {
    let mut delta = 0.0;
    for event in mouse_wheel_event.read() {
        match event.unit {
            MouseScrollUnit::Line => delta += event.y * 10.0,
            MouseScrollUnit::Pixel => delta += event.y,
        }
    }
    if delta == 0.0 {
        return;
    }

    if let Projection::Orthographic(camera_projection) = camera_projection.as_mut() {
        let current_scale = camera_projection.scale;
        camera_projection.scale = (camera_projection.scale + current_scale * delta * 0.01)
            .clamp(MIN_CAMERA_SCALE, MAX_CAMERA_SCALE);
    }
}

fn handle_keyboard(
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
    time: Res<Time>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let (mut camera_transform, mut camera_projection) = camera.into_inner();

    // Camera Zoom
    let mut zoom = 0.0;
    if kb_input.pressed(KeyCode::Equal) || kb_input.pressed(KeyCode::NumpadAdd) {
        zoom -= 1.;
    } else if kb_input.pressed(KeyCode::Minus) || kb_input.pressed(KeyCode::NumpadSubtract) {
        zoom += 1.;
    }

    let mut current_scale = 1.0;
    if let Projection::Orthographic(camera_projection) = camera_projection.as_mut() {
        if zoom != 0.0 {
            camera_projection.scale = (camera_projection.scale
                + (zoom * camera_projection.scale) * 0.9 * time.delta_secs())
            .clamp(MIN_CAMERA_SCALE, MAX_CAMERA_SCALE);
            // eprintln!("Camera Scale {:?}", camera_projection.scale);
        }
        current_scale = camera_projection.scale;
    }

    // Camera Translation
    let mut direction = Vec2::ZERO;

    if kb_input.pressed(KeyCode::ArrowDown) {
        direction.y += 1.;
    }

    if kb_input.pressed(KeyCode::ArrowUp) {
        direction.y -= 1.;
    }

    if kb_input.pressed(KeyCode::ArrowLeft) {
        direction.x += 1.;
    }

    if kb_input.pressed(KeyCode::ArrowRight) {
        direction.x -= 1.;
    }

    if direction.length() > 0.0 {
        let move_delta = -1. * direction * 300. * time.delta_secs() * current_scale;
        camera_transform.translation += move_delta.extend(0.);
        // eprintln!("Camera Translation {:?}", camera_transform.translation);
    }
}

/// Registers materials used for this example.
#[derive(Default)]
struct TextureSupportMaterialsPlugin;

impl Plugin for TextureSupportMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<Texture2dMipMaterial>::default())
            .add_plugins(Material2dPlugin::<Texture2dArrayMipMaterial>::default());
    }
}

/// GPU representation of [`Texture2dMipMaterial`] and [`Texture2dArrayMipMaterial`]
#[derive(Clone, Default, ShaderType)]
struct TextureMipMaterialUniform {
    texture_available: u32,
    mip_level: f32,
    layer_index: u32,
}

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
#[uniform(0, TextureMipMaterialUniform)]
struct Texture2dMipMaterial {
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
    mip_level: u32,
}

impl Material2d for Texture2dMipMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/texture2d_mip_material.wgsl".into())
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl AsBindGroupShaderType<TextureMipMaterialUniform> for Texture2dMipMaterial {
    fn as_bind_group_shader_type(
        &self,
        images: &RenderAssets<GpuImage>,
    ) -> TextureMipMaterialUniform {
        let mut texture_available = 0;
        if let Some(gpu_image) = images.get(self.texture.id()) {
            // Only render the texture if the requested mip exists
            if self.mip_level < gpu_image.mip_level_count {
                texture_available = 1;
            }
        }
        TextureMipMaterialUniform {
            texture_available,
            mip_level: self.mip_level as f32,
            layer_index: 0,
        }
    }
}

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
#[uniform(0, TextureMipMaterialUniform)]
struct Texture2dArrayMipMaterial {
    #[texture(1, dimension = "2d_array")]
    #[sampler(2)]
    texture: Handle<Image>,
    mip_level: u32,
    layer_index: u32,
}

impl Material2d for Texture2dArrayMipMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/texture2d_array_mip_material.wgsl".into())
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl AsBindGroupShaderType<TextureMipMaterialUniform> for Texture2dArrayMipMaterial {
    fn as_bind_group_shader_type(
        &self,
        images: &RenderAssets<GpuImage>,
    ) -> TextureMipMaterialUniform {
        let mut texture_available = 0;
        if let Some(gpu_image) = images.get(self.texture.id()) {
            let layer_count = gpu_image.texture.depth_or_array_layers();
            // Only render the texture if the requested mip and layer exists
            if self.mip_level < gpu_image.mip_level_count && self.layer_index < layer_count {
                texture_available = 1;
            }
        }
        TextureMipMaterialUniform {
            texture_available,
            mip_level: self.mip_level as f32,
            layer_index: self.layer_index,
        }
    }
}
