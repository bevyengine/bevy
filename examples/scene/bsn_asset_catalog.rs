//! Demonstrates the BSN asset catalog: loading named material definitions from
//! a `.bsn` file, applying them to meshes, and serializing assets back to BSN.
//!
//! The catalog file (`assets/scenes/material_catalog.bsn`) defines four named
//! `StandardMaterial` assets with different PBR properties. They are loaded as
//! labeled sub-assets via the asset server:
//!
//!     asset_server.load("scenes/material_catalog.bsn#PolishedMetal")
//!
//! Press S to serialize the loaded materials back to BSN text and print to the
//! console, demonstrating the round-trip capability.

use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;
use bevy_scene2::bsn_asset_catalog::{serialize_assets_to_bsn, CatalogAssetRef};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (check_materials_loaded, save_catalog_on_keypress))
        .run();
}

#[derive(Resource)]
struct CatalogMaterials {
    handles: Vec<(String, Handle<StandardMaterial>)>,
    logged: bool,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let sphere = meshes.add(Sphere::new(0.5).mesh().ico(5).unwrap());

    let catalog_entries: &[(&str, &str)] = &[
        ("PolishedMetal", "Polished Metal"),
        ("BrushedMetal", "Brushed Metal"),
        ("RoughStone", "Rough Stone"),
        ("Plastic", "Plastic"),
    ];

    let spacing = 1.5;
    let offset = (catalog_entries.len() as f32 - 1.0) * spacing / 2.0;
    let mut handles = Vec::new();

    for (i, (catalog_name, display_name)) in catalog_entries.iter().enumerate() {
        let x = i as f32 * spacing - offset;
        let material: Handle<StandardMaterial> =
            asset_server.load(format!("scenes/material_catalog.bsn#{catalog_name}"));

        handles.push((catalog_name.to_string(), material.clone()));

        // Sphere
        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.5, 0.0),
        ));

        // Label
        commands.spawn((
            Text2d::new(*display_name),
            TextFont::from_font_size(14.0),
            Transform::from_xyz(x, -0.3, 0.0),
        ));
    }

    commands.insert_resource(CatalogMaterials {
        handles,
        logged: false,
    });

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
        MeshMaterial3d::<StandardMaterial>::default(),
    ));

    // Lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.7, -FRAC_PI_4)),
    ));

    commands.spawn((
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 500.0,
            rotation: Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: false,
        },
        Transform::default(),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));

    info!("Press S to serialize the loaded materials back to BSN text");
}

/// Log material properties once they finish loading, as verification.
fn check_materials_loaded(
    mut catalog: ResMut<CatalogMaterials>,
    materials: Res<Assets<StandardMaterial>>,
) {
    if catalog.logged {
        return;
    }

    let all_loaded = catalog
        .handles
        .iter()
        .all(|(_, h)| materials.get(h).is_some());
    if !all_loaded {
        return;
    }

    catalog.logged = true;
    info!("All catalog materials loaded:");
    for (name, handle) in &catalog.handles {
        if let Some(mat) = materials.get(handle) {
            info!(
                "  #{name}: metallic={:.2}, roughness={:.2}, reflectance={:.2}",
                mat.metallic, mat.perceptual_roughness, mat.reflectance
            );
        }
    }
}

/// Press S to serialize the loaded materials back to BSN catalog text.
fn save_catalog_on_keypress(
    input: Res<ButtonInput<KeyCode>>,
    catalog: Res<CatalogMaterials>,
    materials: Res<Assets<StandardMaterial>>,
    world: &World,
) {
    if !input.just_pressed(KeyCode::KeyS) {
        return;
    }

    let asset_refs: Vec<_> = catalog
        .handles
        .iter()
        .filter_map(|(name, handle)| {
            materials.get(handle)?;
            Some(CatalogAssetRef {
                name: name.clone(),
                type_id: std::any::TypeId::of::<StandardMaterial>(),
                asset_id: handle.id().untyped(),
            })
        })
        .collect();

    let bsn_text = serialize_assets_to_bsn(world, &asset_refs);
    info!("Serialized catalog to BSN:\n{bsn_text}");
}
