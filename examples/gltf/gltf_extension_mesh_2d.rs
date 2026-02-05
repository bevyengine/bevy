//! Uses glTF extension processing to convert incoming 3d Meshes to 2d Meshes

use bevy::{
    asset::LoadContext,
    gltf::extensions::{GltfExtensionHandler, GltfExtensionHandlers},
    gltf::GltfPlugin,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    prelude::*,
    reflect::TypePath,
    render::render_resource::*,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dKey, Material2dPlugin},
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/custom_gltf_2d.wgsl";

/// This vertex attribute supplies barycentric coordinates for each triangle.
///
/// Each component of the vector corresponds to one corner of a triangle. It's
/// equal to 1.0 in that corner and 0.0 in the other two. Hence, its value in
/// the fragment shader indicates proximity to a corner or the opposite edge.
const ATTRIBUTE_BARYCENTRIC: MeshVertexAttribute =
    MeshVertexAttribute::new("Barycentric", 2137464976, VertexFormat::Float32x3);

fn main() {
    App::new()
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins((
            DefaultPlugins.set(
                GltfPlugin::default()
                    // Map a custom glTF attribute name to a `MeshVertexAttribute`.
                    // The glTF file used here has an attribute name with *two*
                    // underscores: __BARYCENTRIC
                    // One is stripped to do the comparison here.
                    .add_custom_vertex_attribute("_BARYCENTRIC", ATTRIBUTE_BARYCENTRIC),
            ),
            GltfToMesh2dPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/barycentric/barycentric.gltf")),
        ),
        Transform::from_scale(150. * Vec3::ONE),
    ));
    commands.spawn(Camera2d);
}

struct GltfToMesh2dPlugin;

impl Plugin for GltfToMesh2dPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_family = "wasm")]
        bevy::tasks::block_on(async {
            app.world_mut()
                .resource_mut::<GltfExtensionHandlers>()
                .0
                .write()
                .await
                .push(Box::new(GltfExtensionHandlerToMesh2d))
        });
        #[cfg(not(target_family = "wasm"))]
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write_blocking()
            .push(Box::new(GltfExtensionHandlerToMesh2d));

        app.add_plugins(Material2dPlugin::<CustomMaterial>::default());
    }
}

#[derive(Default, Clone)]
struct GltfExtensionHandlerToMesh2d;

impl GltfExtensionHandler for GltfExtensionHandlerToMesh2d {
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler> {
        Box::new((*self).clone())
    }

    fn on_spawn_mesh_and_material(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _primitive: &gltf::Primitive,
        _mesh: &gltf::Mesh,
        _material: &gltf::Material,
        entity: &mut EntityWorldMut,
    ) {
        if let Some(mesh3d) = entity.get::<Mesh3d>()
            && let Some(_) = entity.get::<MeshMaterial3d<StandardMaterial>>()
        {
            let material_handle =
                load_context.add_labeled_asset("AColorMaterial".to_string(), CustomMaterial {});
            let mesh_handle = mesh3d.0.clone();
            entity
                .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>()
                .insert((Mesh2d(mesh_handle), MeshMaterial2d(material_handle.clone())));
        }
    }
}

/// This custom material uses barycentric coordinates from
/// `ATTRIBUTE_BARYCENTRIC` to shade a white border around each triangle. The
/// thickness of the border is animated using the global time shader uniform.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {}

impl Material2d for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            ATTRIBUTE_BARYCENTRIC.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
