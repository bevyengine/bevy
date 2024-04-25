//! Shows how to animate material properties

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_materials)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(3.0, 1.0, 3.0)
                .looking_at(Vec3::new(0.0, -0.5, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2_000.0,
        },
    ));

    let cube = meshes.add(Cuboid::new(0.5, 0.5, 0.5));

    const GOLDEN_ANGLE: f32 = 137.507_77;

    let mut hsla = Hsla::hsl(0.0, 1.0, 0.5);
    for x in -1..2 {
        for z in -1..2 {
            commands.spawn(PbrBundle {
                mesh: cube.clone(),
                material: materials.add(Color::from(hsla)),
                transform: Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
                ..default()
            });
            hsla = hsla.rotate_hue(GOLDEN_ANGLE);
        }
    }
}

fn animate_materials(
    material_handles: Query<&Handle<StandardMaterial>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for material_handle in material_handles.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            if let Color::Hsla(ref mut hsla) = material.base_color {
                *hsla = hsla.rotate_hue(time.delta_seconds() * 100.0);
            }
        }
    }
}
