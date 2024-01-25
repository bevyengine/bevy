//! Shows how to animate material properties

use bevy::prelude::*;
use bevy_internal::utils::HashSet;
use core::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_materials, make_materials_unique))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(3.0, 1.0, 3.0)
                .looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1500.0,
        },
    ));

    let helmet = asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0");
    for x in -2..3 {
        for z in -2..3 {
            commands.spawn(SceneBundle {
                scene: helmet.clone(),
                transform: Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
                ..default()
            });
        }
    }
}

fn animate_materials(
    material_handles: Query<&Handle<StandardMaterial>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (i, material_handle) in material_handles.iter().enumerate() {
        if let Some(material) = materials.get_mut(material_handle) {
            let color = Color::hsl(
                ((i as f32 * 2.345 + time.elapsed_seconds_wrapped()) * 100.0) % 360.0,
                1.0,
                0.5,
            );
            material.base_color = color;
            material.emissive = color;
        }
    }
}

/// This is needed because by default assets are loaded with shared materials
/// But we want to animate every helmet independently of the others, so we must duplicate the materials
fn make_materials_unique(
    mut material_handles: Query<&mut Handle<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ran: Local<bool>,
) {
    if *ran {
        return;
    }
    let mut set = HashSet::new();
    for mut material_handle in material_handles.iter_mut() {
        if set.contains(&material_handle.id()) {
            let material = materials.get(&*material_handle).unwrap().clone();
            *material_handle = materials.add(material);
        } else {
            set.insert(material_handle.id());
        }
        *ran = true;
    }
}
