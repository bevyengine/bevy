//! A shader that uses "shaders defs", which selectively toggle parts of a shader.

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
        },
    },
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // blue cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(-1.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {
            color: Color::BLUE,
            is_red: false,
        }),
        ..default()
    });

    // red cube (with green color overridden by the IS_RED "shader def")
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(1.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {
            color: Color::GREEN,
            is_red: true,
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/shader_defs.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.is_red {
            let fragment = descriptor.fragment.as_mut().unwrap();
            fragment.shader_defs.push("IS_RED".into());
        }
        Ok(())
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(CustomMaterialKey)]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
    is_red: bool,
}

// This key is used to identify a specific permutation of this material pipeline.
// In this case, we specialize on whether or not to configure the "IS_RED" shader def.
// Specialization keys should be kept as small / cheap to hash as possible,
// as they will be used to look up the pipeline for each drawn entity with this material type.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct CustomMaterialKey {
    is_red: bool,
}

impl From<&CustomMaterial> for CustomMaterialKey {
    fn from(material: &CustomMaterial) -> Self {
        Self {
            is_red: material.is_red,
        }
    }
}
