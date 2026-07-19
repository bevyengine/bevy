//! Demonstrates pipeline-overridable constants using posterization.
//!
//! The same shader is compiled into three distinct pipeline variants by setting the `LEVELS`
//! `override` constant to different values at pipeline creation time. Each quad on screen uses a
//! different variant, producing 2, 4, and 8 discrete color steps from the same smooth gradient.
//!
//! This is similar to `shader_defs` but operates at the GPU compiler level: constants are
//! substituted into the shader source before compilation, allowing the driver to optimize each
//! variant independently. Unlike uniforms, pipeline constants cannot change at draw time.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dKey, Material2dPipeline, Material2dPlugin},
};

const SHADER_ASSET_PATH: &str = "shaders/pipeline_constants.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<PosterizeMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PosterizeMaterial>>,
) {
    commands.spawn(Camera2d);

    let quad = meshes.add(Rectangle::new(200.0, 150.0));

    for (i, levels) in [2u32, 4, 8].into_iter().enumerate() {
        let x = (i as f32 - 1.0) * 220.0;

        commands.spawn((
            Mesh2d(quad.clone()),
            MeshMaterial2d(materials.add(PosterizeMaterial { levels })),
            Transform::from_xyz(x, 20.0, 0.0),
        ));

        commands.spawn((
            Text2d::new(format!("{levels} levels")),
            Transform::from_xyz(x, -65.0, 0.0),
        ));
    }
}

/// A material that posterizes a color gradient using a pipeline-overridable constant.
///
/// Each distinct `levels` value produces a separate compiled pipeline variant.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(PosterizeMaterialKey)]
struct PosterizeMaterial {
    /// Number of discrete color steps. Compiled into the shader as a pipeline constant.
    levels: u32,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PosterizeMaterialKey {
    levels: u32,
}

impl From<&PosterizeMaterial> for PosterizeMaterialKey {
    fn from(m: &PosterizeMaterial) -> Self {
        Self { levels: m.levels }
    }
}

impl Material2d for PosterizeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .fragment
            .as_mut()
            .unwrap()
            .constants
            .push(("LEVELS".into(), key.bind_group_data.levels as f64));
        Ok(())
    }
}
