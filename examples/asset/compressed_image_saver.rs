//! Demonstrates compressing textures and generating mipmaps using `CompressedImageSaver`.

use bevy::{
    camera::Hdr,
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    mesh::SphereKind,
    prelude::*,
};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

fn main() {
    download_assets().expect("failed to download GroundSand005 textures");

    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            mode: AssetMode::Processed,
            ..default()
        }))
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_systems(Startup, spawn_scene)
        .add_systems(Update, rotate)
        .run();
}

fn spawn_scene(
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    // See .meta files at https://github.com/bevyengine/bevy_asset_files/tree/main/GroundSand005
    let orm = asset_server.load("textures/GroundSand005/GroundSand005_ORM_2K.png");
    let sphere_material = StandardMaterial {
        base_color_texture: Some(
            asset_server.load("textures/GroundSand005/GroundSand005_COL_2K.jpg"),
        ),
        perceptual_roughness: 1.0,
        metallic_roughness_texture: Some(orm.clone()),
        normal_map_texture: Some(
            asset_server.load("textures/GroundSand005/GroundSand005_NRM_2K.jpg"),
        ),
        occlusion_texture: Some(orm),
        parallax_mapping_method: ParallaxMappingMethod::Relief { max_steps: 4 },
        depth_map: Some(asset_server.load("textures/GroundSand005/GroundSand005_DISP_2K.jpg")),
        ..Default::default()
    };

    let diffuse_env_map = asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2");
    let specular_env_map = asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");

    let floor_mesh = meshes.add(Circle::new(4.0).mesh_builder().resolution(128).build());

    let sphere_mesh = meshes.add(
        Sphere::new(1.0)
            .mesh_builder()
            .kind(SphereKind::Ico { subdivisions: 50 })
            .build()
            .with_generated_tangents()
            .unwrap(),
    );

    commands.spawn_scene_list(bsn_list! [
        (
            Mesh3d(floor_mesh)
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::WHITE))
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        ),
        (
            Mesh3d(sphere_mesh)
            MeshMaterial3d::<StandardMaterial>(asset_value(sphere_material))
            Transform::from_xyz(0.0, 1.0, 0.0)
            Rotating
        ),
        (
            DirectionalLight {
                illuminance: 7300.0,
                shadow_maps_enabled: true,
            }
            template_value(Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y))
            template_value(CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 20.0,
                ..default()
            }.build())
        ),
        (
            Camera3d
            Hdr
            template_value(Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
            EnvironmentMapLight {
                diffuse_map: diffuse_env_map,
                specular_map: specular_env_map,
                intensity: 1200.0,
            }
        )
    ]);
}

#[derive(Component, Default, Clone)]
struct Rotating;

fn rotate(mut query: Query<&mut Transform, With<Rotating>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}

// HACK: this function isn't relevant to asset processing in general.
// We download the source textures at runtime rather than checking them
// into the repo because they're large files we don't want to store in
// version control. Asset processing needs to iterate through the contents of
// an asset source to discover files to process, and the web asset source
// doesn't support iterating through its contents, so we can't simply point
// the processor at a remote URL and have to fetch the files to a local
// directory first.
fn download_assets() -> io::Result<()> {
    let asset_files = &[
        "GroundSand005_COL_2K.jpg",
        "GroundSand005_COL_2K.jpg.meta",
        "GroundSand005_NRM_2K.jpg",
        "GroundSand005_NRM_2K.jpg.meta",
        "GroundSand005_DISP_2K.jpg",
        "GroundSand005_DISP_2K.jpg.meta",
        "GroundSand005_ORM_2K.png",
        "GroundSand005_ORM_2K.png.meta",
    ];

    let dir = Path::new("assets/textures/GroundSand005");
    fs::create_dir_all(dir)?;

    for file in asset_files {
        let dest = dir.join(file);
        if dest.exists() {
            continue;
        }

        let url =
            format!("https://github.com/bevyengine/bevy_asset_files/raw/main/GroundSand005/{file}");
        info!("Downloading {url}");

        let mut response = ureq::get(&url)
            .call()
            .map_err(|e| io::Error::other(e.to_string()))?;
        let mut bytes = Vec::new();
        response.body_mut().as_reader().read_to_end(&mut bytes)?;
        fs::write(&dest, &bytes)?;
    }

    Ok(())
}
