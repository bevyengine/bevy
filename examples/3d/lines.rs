//! Create a custom material to draw basic lines in 3D

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{
            AsBindGroup, PolygonMode, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/line_material.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<LineMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    // Spawn a list of lines with start and end points for each lines
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(LineList {
            lines: vec![
                (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)),
                (Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            ],
        }),
        transform: Transform::from_xyz(-1.5, 0.0, 0.0),
        material: materials.add(LineMaterial {
            color: LinearRgba::GREEN,
        }),
        ..default()
    });

    // Spawn a line strip that goes from point to point
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(LineStrip {
            points: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ],
        }),
        transform: Transform::from_xyz(0.5, 0.0, 0.0),
        material: materials.add(LineMaterial {
            color: LinearRgba::BLUE,
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Asset, TypePath, Default, AsBindGroup, Debug, Clone)]
struct LineMaterial {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material for LineMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // This is the important part to tell bevy to render this material as a line between vertices
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}

/// A list of lines with a start and end position
#[derive(Debug, Clone)]
struct LineList {
    lines: Vec<(Vec3, Vec3)>,
}

impl From<LineList> for Mesh {
    fn from(line: LineList) -> Self {
        let vertices: Vec<_> = line.lines.into_iter().flat_map(|(a, b)| [a, b]).collect();

        Mesh::new(
            // This tells wgpu that the positions are list of lines
            // where every pair is a start and end point
            PrimitiveTopology::LineList,
            RenderAssetUsages::RENDER_WORLD,
        )
        // Add the vertices positions as an attribute
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    }
}

/// A list of points that will have a line drawn between each consecutive points
#[derive(Debug, Clone)]
struct LineStrip {
    points: Vec<Vec3>,
}

impl From<LineStrip> for Mesh {
    fn from(line: LineStrip) -> Self {
        Mesh::new(
            // This tells wgpu that the positions are a list of points
            // where a line will be drawn between each consecutive point
            PrimitiveTopology::LineStrip,
            RenderAssetUsages::RENDER_WORLD,
        )
        // Add the point positions as an attribute
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, line.points)
    }
}
