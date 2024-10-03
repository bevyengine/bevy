//! A simple 3D scene to demonstrate mesh picking.

use bevy::{
    color::palettes::{
        css::{BLUE, GREEN, PINK, RED},
        tailwind::CYAN_400,
    },
    picking::backend::PointerHits,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SceneMaterials>()
        .add_systems(Startup, setup)
        .add_systems(Update, on_mesh_hover)
        .run();
}

#[derive(Resource, Default)]
struct SceneMaterials {
    pub white: Handle<StandardMaterial>,
    pub hover: Handle<StandardMaterial>,
    pub pressed: Handle<StandardMaterial>,
}

/// Set up a simple 3D scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_materials: ResMut<SceneMaterials>,
) {
    scene_materials.white = materials.add(Color::WHITE);
    scene_materials.hover = materials.add(Color::from(CYAN_400));
    scene_materials.pressed = materials.add(Color::from(GREEN));

    // Circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Pickable::default(),
    ));

    // Cube
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 0.5, 0.0),
            Pickable::default(),
        ))
        .observe(
            |trigger: Trigger<Pointer<Over>>,
             scene_materials: Res<SceneMaterials>,
             mut query: Query<&mut MeshMaterial3d<StandardMaterial>>| {
                if let Ok(mut material) = query.get_mut(trigger.entity()) {
                    material.0 = scene_materials.hover.clone();
                }
            },
        )
        .observe(
            |trigger: Trigger<Pointer<Out>>,
             scene_materials: Res<SceneMaterials>,
             mut query: Query<&mut MeshMaterial3d<StandardMaterial>>| {
                if let Ok(mut material) = query.get_mut(trigger.entity()) {
                    material.0 = scene_materials.white.clone();
                }
            },
        );

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn change_material_on(
    mut pointer_hits: EventReader<PointerHits>,
    scene_materials: Res<SceneMaterials>,
    mut query: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    for hit in pointer_hits.read() {
        for (entity, _) in hit.picks.iter() {
            if let Ok(mut material) = query.get_mut(*entity) {
                material.0 = scene_materials.hover.clone();
            }
        }
    }
}

fn on_mesh_hover(
    mut pointer_hits: EventReader<PointerHits>,
    meshes: Query<Entity, With<Mesh3d>>,
    mut gizmos: Gizmos,
) {
    for hit in pointer_hits.read() {
        let mesh_hits = hit
            .picks
            .iter()
            .filter_map(|(entity, hit)| meshes.get(*entity).map(|_| hit).ok());

        for hit in mesh_hits {
            let (Some(point), Some(normal)) = (hit.position, hit.normal) else {
                return;
            };
            gizmos.sphere(Isometry3d::from_translation(point), 0.05, RED);
            gizmos.arrow(point, point + normal * 0.5, PINK);
        }
    }
}
