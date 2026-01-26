//! Loads and renders a glTF file as a scene with run-time generated mip maps and optional texture compression.

use std::path::PathBuf;

use argh::FromArgs;
use bevy::{asset::UnapprovedPathMode, prelude::*};
use mipmap_generator::{
    generate_mipmaps, MipmapGeneratorDebugTextPlugin, MipmapGeneratorPlugin,
    MipmapGeneratorSettings,
};

#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// compress textures (requires compress feature)
    #[argh(switch)]
    compress: bool,
    /// if set, raw compressed image data will be cached in this directory. Images that are not BCn compressed are not cached.
    #[argh(switch)]
    cache: bool,
    /// if low_quality is set, only 0.5 byte/px formats will be used (BC1, BC4) unless the alpha channel is in use, then BC3 will be used. When low quality is set, compression is generally faster than CompressionSpeed::UltraFast and CompressionSpeed is ignored.
    #[argh(switch)]
    low_quality: bool,
}

fn main() {
    let args: Args = argh::from_env();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)))
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Needed to load from the parent bevy assets folder
            unapproved_path_mode: UnapprovedPathMode::Allow,
            ..default()
        }))
        .insert_resource(MipmapGeneratorSettings {
            // Manually setting anisotropic filtering to 16x
            anisotropic_filtering: 16,
            compression: args.compress.then(Default::default),
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
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.0, 0.2, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-1.0, 2.0, -3.0),
    ));

    commands.spawn(SceneRoot(
        asset_server.load(
            // This seems to be the correct path but bevy doesn't resolve it.
            GltfSubassetName::Scene(0)
                .from_asset("../../../../assets/models/FlightHelmet/FlightHelmet.gltf"),
        ),
    ));
}
