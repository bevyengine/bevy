//! Create a custom material to draw basic lines in 3D

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{MeshVertexBufferLayout, PrimitiveTopology},
        render_resource::{
            AsBindGroup, PolygonMode, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<LineMaterial>::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    // Spawn a list of lines with start and end points for each lines
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(LineList {
            lines: vec![
                (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)),
                (Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            ],
        })),
        transform: Transform::from_xyz(-1.5, 0.0, 0.0),
        material: materials.add(LineMaterial {
            color: Color::GREEN,
        }),
        ..default()
    });

    // Spawn a line strip that goes from point to point
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(LineStrip {
            points: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
            ],
        })),
        transform: Transform::from_xyz(0.5, 0.0, 0.0),
        material: materials.add(LineMaterial { color: Color::BLUE }),
        ..default()
    });

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Default, AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "050ce6ac-080a-4d8c-b6b5-b5bab7560d8f"]
struct LineMaterial {
    #[uniform(0)]
    color: Color,
}

impl Material for LineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/line_material.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // This is the important part to tell bevy to render this material as a line between vertices
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}

/// A list of lines with a start and end position
#[derive(Debug, Clone)]
pub struct LineList {
    pub lines: Vec<(Vec3, Vec3)>,
}

impl From<LineList> for Mesh {
    fn from(line: LineList) -> Self {
        let mut vertices = vec![];
        let mut normals = vec![];
        for (start, end) in line.lines {
            vertices.push(start.to_array());
            vertices.push(end.to_array());
            normals.push(Vec3::ZERO.to_array());
            normals.push(Vec3::ZERO.to_array());
        }

        // This tells wgpu that the positions are list of lines
        // where every pair is a start and end point
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        // Normals are currently required by bevy, but they aren't used by the [`LineMaterial`]
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh
    }
}

/// A list of points that will have a line drawn between each consecutive points
#[derive(Debug, Clone)]
pub struct LineStrip {
    pub points: Vec<Vec3>,
}

impl From<LineStrip> for Mesh {
    fn from(line: LineStrip) -> Self {
        let mut vertices = vec![];
        let mut normals = vec![];
        for pos in line.points {
            vertices.push(pos.to_array());
            normals.push(Vec3::ZERO.to_array());
        }

        // This tells wgpu that the positions are a list of points
        // where a line will be drawn between each consecutive point
        let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        // Normals are currently required by bevy, but they aren't used by the [`LineMaterial`]
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh
    }
}
