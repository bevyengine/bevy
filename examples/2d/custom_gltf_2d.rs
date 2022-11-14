//! Renders a glTF mesh in 2D with a custom vertex attribute.

use bevy::gltf::GltfPlugin;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayout};
use bevy::render::render_resource::*;
use bevy::sprite::{
    Material2d, Material2dKey, Material2dPlugin, MaterialMesh2dBundle, Mesh2dHandle,
};

const ATTRIBUTE_BARYCENTRIC: MeshVertexAttribute =
    MeshVertexAttribute::new("Barycentric", 2137464976, VertexFormat::Float32x3);

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(
            DefaultPlugins.set(
                GltfPlugin::default()
                    .add_custom_vertex_attribute("_BARYCENTRIC", ATTRIBUTE_BARYCENTRIC),
            ),
        )
        .add_plugin(Material2dPlugin::<CustomMaterial>::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let mesh = asset_server.load("models/barycentric/barycentric.glb#Mesh0/Primitive0");

    commands.spawn(Camera2dBundle { ..default() });
    commands.spawn(MaterialMesh2dBundle {
        mesh: Mesh2dHandle(mesh),
        material: materials.add(CustomMaterial {}),
        transform: Transform::from_scale(150.0 * Vec3::ONE),
        ..default()
    });
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
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
