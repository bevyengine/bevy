//! Renders a glTF mesh in 2D with a custom vertex attribute.

use bevy::gltf::GltfPlugin;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayout};
use bevy::render::render_resource::*;
use bevy::sprite::{
    Material2d, Material2dKey, Material2dPlugin, MaterialMesh2dBundle, Mesh2dHandle,
};

/// This vertex attribute supplies barycentric coordinates for each triangle.
/// Each component of the vector corresponds to one corner of a triangle. It's
/// equal to 1.0 in that corner and 0.0 in the other two. Hence, its value in
/// the fragment shader indicates proximity to a corner or the opposite edge.
const ATTRIBUTE_BARYCENTRIC: MeshVertexAttribute =
    MeshVertexAttribute::new("Barycentric", 2137464976, VertexFormat::Float32x3);

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins((
            DefaultPlugins.set(
                GltfPlugin::default()
                    // Map a custom glTF attribute name to a `MeshVertexAttribute`.
                    .add_custom_vertex_attribute("_BARYCENTRIC", ATTRIBUTE_BARYCENTRIC),
            ),
            Material2dPlugin::<CustomMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // Add a mesh loaded from a glTF file. This mesh has data for `ATTRIBUTE_BARYCENTRIC`.
    let mesh = asset_server.load("models/barycentric/barycentric.gltf#Mesh0/Primitive0");
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(mesh),
        material: materials.add(CustomMaterial {}),
        transform: Transform::from_scale(150.0 * Vec3::ONE),
        ..default()
    });

    // Add a camera
    commands.spawn(Camera2dBundle { ..default() });
}

/// This custom material uses barycentric coordinates from
/// `ATTRIBUTE_BARYCENTRIC` to shade a white border around each triangle. The
/// thickness of the border is animated using the global time shader uniform.
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "50ffce9e-1582-42e9-87cb-2233724426c0"]
struct CustomMaterial {}

impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_gltf_2d.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/custom_gltf_2d.wgsl".into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            ATTRIBUTE_BARYCENTRIC.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
