use std::{f32::consts::PI, path::PathBuf};

use argh::FromArgs;
use bevy::{
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};
use mipmap_generator::{
    generate_mipmaps, MipmapGeneratorDebugTextPlugin, MipmapGeneratorPlugin,
    MipmapGeneratorSettings,
};

#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// if set, raw compressed image data will be cached in this directory. Images that are not BCn compressed are not cached.
    #[argh(switch)]
    cache: bool,
    /// if low_quality is set, only 0.5 byte/px formats will be used (BC1, BC4) unless the alpha channel is in use, then BC3 will be used. When low quality is set, compression is generally faster than CompressionSpeed::UltraFast and CompressionSpeed is ignored.
    #[argh(switch)]
    low_quality: bool,
}

fn main() {
    let args: Args = argh::from_env();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(MipmapGeneratorSettings {
            compression: Some(Default::default()),
            compressed_image_data_cache_path: if args.cache {
                Some(PathBuf::from("compressed_texture_cache"))
            } else {
                None
            },
            low_quality: args.low_quality,
            ..default()
        })
        .add_systems(Startup, setup)
        // Add MipmapGeneratorPlugin after default plugins
        .add_plugins((MipmapGeneratorPlugin, MipmapGeneratorDebugTextPlugin))
        // Add material types to be converted
        .add_systems(Update, generate_mipmaps::<StandardMaterial>);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let image_r = create_test_image(2048, -0.8, 0.156, 1);
    let mut mat_r = StandardMaterial::from(images.add(image_r));
    mat_r.unlit = true;

    let image_rg = create_test_image(2048, -0.8, 0.156, 2);
    let mut mat_rg = StandardMaterial::from(images.add(image_rg));
    mat_rg.unlit = true;

    let image_rgba = create_test_image(2048, -0.8, 0.156, 4);
    let mut mat_rgba = StandardMaterial::from(images.add(image_rgba));
    mat_rgba.unlit = true;

    let plane_h = meshes.add(Plane3d::default().mesh().size(20.0, 30.0));

    // planes
    commands.spawn((
        Mesh3d(plane_h.clone()),
        MeshMaterial3d(materials.add(mat_r)),
        Transform::from_xyz(-3.0, 0.0, 0.0).with_rotation(Quat::from_rotation_z(-PI * 0.5)),
    ));
    commands.spawn((
        Mesh3d(plane_h.clone()),
        MeshMaterial3d(materials.add(mat_rg)),
        Transform::from_xyz(3.0, 0.0, 0.0).with_rotation(Quat::from_rotation_z(PI * 0.5)),
    ));
    commands.spawn((
        Mesh3d(plane_h.clone()),
        MeshMaterial3d(materials.add(mat_rgba)),
        Transform::from_xyz(0.0, -3.0, 0.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn create_test_image(size: u32, cx: f32, cy: f32, channels: u32) -> Image {
    let data: Vec<u8> = (0..size * size)
        .flat_map(|id| {
            let mut x = 4.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
            let mut y = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
            let mut count = 0;
            while count < 0xFF && x * x + y * y < 4.0 {
                let old_x = x;
                x = x * x - y * y + cx;
                y = 2.0 * old_x * y + cy;
                count += 1;
            }
            let mut values = vec![0xFF - (count * 2) as u8];
            if channels > 1 {
                values.push(0xFF - (count * 5) as u8);
            }
            if channels > 2 {
                values.push(0xFF - (count * 13) as u8);
                values.push(u8::MAX);
            }
            values
        })
        .collect();

    Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size,
                height: size,
                ..default()
            },
            dimension: TextureDimension::D2,
            format: if channels == 1 {
                TextureFormat::R8Unorm
            } else if channels == 2 {
                TextureFormat::Rg8Unorm
            } else {
                TextureFormat::Rgba8UnormSrgb
            },
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        data: Some(data),
        ..Default::default()
    }
}
