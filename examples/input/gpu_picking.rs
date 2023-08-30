//! This example shows how to use the gpu picking api.
//!
//! Gpu picking is a way to generate a texture of all the rendered entities and
//! use this texture to determine exactly which entity is under the mouse.

use bevy::prelude::*;
use bevy_internal::{
    reflect::{TypePath, TypeUuid},
    render::{
        picking::{GpuPickingCamera, GpuPickingMesh, GpuPickingPlugin},
        render_resource::{AsBindGroup, ShaderRef},
    },
    window::PresentMode,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            MaterialPlugin::<GpuPickingMaterial>::default(),
            // Add the plugin
            GpuPickingPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse_picking, move_cube))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<GpuPickingMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // opaque cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        // Add this component to any mesh that you want to be able to pick
        GpuPickingMesh,
    ));

    // alpha mask cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                alpha_mode: AlphaMode::Mask(1.0),
                base_color_texture: Some(asset_server.load("branding/icon.png")),
                ..default()
            }),
            transform: Transform::from_xyz(1.0, 0.5, 0.0),
            ..default()
        },
        GpuPickingMesh,
    ));

    // transparent cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgba(0.8, 0.7, 0.6, 0.5).into()),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        },
        GpuPickingMesh,
    ));

    // cube with custom material
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            transform: Transform::from_xyz(2.0, 0.5, 0.0),
            material: custom_materials.add(GpuPickingMaterial {
                color: Color::GREEN,
            }),
            ..default()
        },
        GpuPickingMesh,
    ));

    // This cube will move from left to right. It shows that picking works correctly when things are moving.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 1.0),
            ..default()
        },
        GpuPickingMesh,
        MoveCube,
    ));

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        GpuPickingCamera::default(),
    ));
}

fn mouse_picking(
    mut cursor_moved: EventReader<CursorMoved>,
    gpu_picking_cameras: Query<&GpuPickingCamera>,
    material_handle: Query<(
        Option<&Handle<StandardMaterial>>,
        Option<&Handle<GpuPickingMaterial>>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<GpuPickingMaterial>>,
    mut hovered: Local<Option<Entity>>,
) {
    // Sets the color of the given entity
    let mut set_color = |entity, color: Color| {
        let (std_handle, custom_handle) = material_handle.get(entity).expect("Entity should exist");
        if let Some(material) = std_handle.and_then(|h| materials.get_mut(h)) {
            let a = material.base_color.a();
            material.base_color = color.with_a(a);
        };
        if let Some(material) = custom_handle.and_then(|h| custom_materials.get_mut(h)) {
            let a = material.color.a();
            material.color = color.with_a(a);
        };
    };

    let Some(moved_event) = cursor_moved.iter().last() else { return; };
    let mouse_position = moved_event.position.as_uvec2();

    for gpu_picking_camera in &gpu_picking_cameras {
        // This will read the entity texture and get the entity that is at the given position
        if let Some(entity) = gpu_picking_camera.get_entity(mouse_position) {
            if let Some(hovered) = *hovered {
                if entity != hovered {
                    set_color(hovered, Color::BLUE);
                }
            }
            set_color(entity, Color::RED);
            *hovered = Some(entity);
        } else {
            if let Some(hovered) = *hovered {
                set_color(hovered, Color::BLUE);
            }
            *hovered = None;
        }
    }
}

// You can also use a custom material with it, you just need to make sure it correctly outputs the entity id
// See assets/shaders/gpu_picking_material.wgsl for more information
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "fb9ea5e0-316d-4992-852b-aa1faa2a5a0d"]
pub struct GpuPickingMaterial {
    #[uniform(0)]
    color: Color,
}

impl Material for GpuPickingMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/gpu_picking_material.wgsl".into()
    }
}

#[derive(Component)]
struct MoveCube;

// Moves a mesh from left to right
// Used to show that picking works even if things are moving
fn move_cube(
    mut q: Query<&mut Transform, With<MoveCube>>,
    time: Res<Time>,
    mut move_left: Local<bool>,
) {
    for mut t in &mut q {
        t.translation.x += if *move_left {
            -time.delta_seconds()
        } else {
            time.delta_seconds()
        };
        if t.translation.x >= 2.0 || t.translation.x <= -2.0 {
            *move_left = !*move_left;
        }
    }
}
