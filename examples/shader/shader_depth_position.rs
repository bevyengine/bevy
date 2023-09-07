//! A shader and a material that uses it.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_internal::core_pipeline::prepass::DepthPrepass;

pub const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(8695250969165824);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

const SHADER_CODE: &str = r"
#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_view_bindings  view
#import bevy_pbr::depth_functions depth_to_view_space_position, depth_to_world_position, depth_to_world_position_two
#import bevy_pbr::prepass_utils

@fragment
fn fragment(
    mesh: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let depth = bevy_pbr::prepass_utils::prepass_depth(mesh.position, 0u);
    let frag_coord = mesh.position;
    let uv = frag_coord.xy;
    let world_position = depth_to_world_position_two(uv, depth, view.inverse_projection, view.world_position);
        // let view_pos = depth_to_view_space_position(depth, uv);
    return vec4(world_position / 10.0, 1.0);
    // return material.color * textureSample(base_color_texture, base_color_sampler, mesh.uv);
}

";

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    asset_server: Res<AssetServer>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    shaders.insert(SHADER_HANDLE, Shader::from_wgsl(SHADER_CODE, file!()));

    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {}),
        ..default()
    });

    // plane
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(CustomMaterial {}),
        ..default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        DepthPrepass,
    ));
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HANDLE.into()
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CustomMaterial {}
