//! This example shows various ways to configure texture materials in 3D.

use std::collections::HashMap;
use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_internal::render::{
    mesh::{Mesh, VertexAttributeValues},
    render_resource::{AddressMode, SamplerDescriptor},
};

// Update a mesh's UVs so that the applied texture tiles with the given `number_of_tiles`.
pub fn update_mesh_uvs_for_number_of_tiles(mesh: &mut Mesh, number_of_tiles: (f32, f32)) {
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs {
            uv[0] *= number_of_tiles.0;
            uv[1] *= number_of_tiles.1;
        }
    }
}

// Update a quad's UVs so that the applied texture tiles with the calculated number of tiles,
// given the intended size of the texture in bevy units.
pub fn update_quad_uvs_for_world_space_texture_size(
    mesh: &mut Mesh,
    world_space_texture_size: (f32, f32),
) {
    let mut attributes = mesh.attributes_mut().collect::<HashMap<_, _>>();
    if let (
        Some(VertexAttributeValues::Float32x2(uvs)),
        Some(VertexAttributeValues::Float32x3(positions)),
    ) = (
        attributes.remove(&Mesh::ATTRIBUTE_UV_0.id),
        attributes.remove(&Mesh::ATTRIBUTE_POSITION.id),
    ) {
        for (position, uv) in positions.iter().zip(uvs) {
            uv[0] = position[0] / world_space_texture_size.0;
            // If you are using this to change the UVs of a shape::Plane, change this to -position[2]
            uv[1] = -position[1] / world_space_texture_size.1;
        }
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // This is needed for tiling textures. If you want to add tiled textures to your project, don't forget this!
                .set(ImagePlugin {
                    default_sampler: SamplerDescriptor {
                        address_mode_u: AddressMode::Repeat,
                        address_mode_v: AddressMode::Repeat,
                        address_mode_w: AddressMode::Repeat,
                        ..default()
                    },
                }),
        )
        .add_startup_system(setup)
        .run();
}

/// sets up a scene with textured entities
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // load a texture and retrieve its aspect ratio
    let texture_handle = asset_server.load("branding/bevy_logo_dark_big.png");
    let aspect = 0.25;

    let quad_width = 8.0;

    // this material renders the texture normally
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // this material modulates the texture to make it red (and slightly transparent)
    let red_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 0.0, 0.0, 0.5),
        base_color_texture: Some(texture_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // and lets make this one blue! (and also slightly transparent)
    let blue_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgba(0.0, 0.0, 1.0, 0.5),
        base_color_texture: Some(texture_handle),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // textured quad - normal
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
            quad_width,
            quad_width * aspect,
        )))),
        material: material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 1.5)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // textured quad - modulated (with texture tiling)
    // tile twice in the X-direction, and thrice in the Y.
    let mut red_tiled_texture_mesh =
        Mesh::from(shape::Quad::new(Vec2::new(quad_width, quad_width * aspect)));
    update_mesh_uvs_for_number_of_tiles(&mut red_tiled_texture_mesh, (2.0, 3.0));
    commands.spawn(PbrBundle {
        mesh: meshes.add(red_tiled_texture_mesh),
        material: red_material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // textured quad - modulated (with texture tiling)
    // make the bevy logo take up (1.0, 0.25) units, and tile.
    let mut blue_tiled_texture_mesh =
        Mesh::from(shape::Quad::new(Vec2::new(quad_width, quad_width * aspect)));
    update_plane_uvs_for_world_space_texture_size(&mut blue_tiled_texture_mesh, (1.0, 0.25));
    commands.spawn(PbrBundle {
        mesh: meshes.add(blue_tiled_texture_mesh),
        material: blue_material_handle,
        transform: Transform::from_xyz(0.0, 0.0, -1.5)
            .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3.0, 5.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
