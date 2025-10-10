//! A shader that uses the WESL shading language.

use bevy::asset::embedded_asset;
use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::{ShaderDefVal, ShaderRef},
};

/// This example uses shader source files from the assets subdirectory
const FRAGMENT_SHADER_ASSET_PATH: &str =
    "embedded://shader_material_wesl/files/custom_material.wesl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<CustomMaterial>::default(),
            CustomMaterialPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

/// A plugin that loads the custom material shader
pub struct CustomMaterialPlugin;

/// An example utility shader that is used by the custom material
#[expect(
    dead_code,
    reason = "used to kept a strong handle, shader is referenced by the material"
)]
#[derive(Resource)]
struct UtilityShader(Handle<Shader>);

#[expect(
    dead_code,
    reason = "used to kept a strong handle, shader is referenced by the material"
)]
#[derive(Resource)]
struct VertexAttrShader(Handle<Shader>);

#[expect(
    dead_code,
    reason = "used to kept a strong handle, shader is referenced by the material"
)]
#[derive(Resource)]
struct CustomMaterialImportShader(Handle<Shader>);

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "examples/shader", "files/custom_material.wesl");
        embedded_asset!(app, "examples/shader", "files/vertex_attr.wesl");
        embedded_asset!(app, "examples/shader", "files/util.wesl");

        let asset_server = app.world_mut().resource_mut::<AssetServer>();
        let utils_handle =
            asset_server.load::<Shader>("embedded://shader_material_wesl/files/util.wesl");
        let vertex_attr_handle =
            asset_server.load::<Shader>("embedded://shader_material_wesl/files/vertex_attr.wesl");
        let custom_material_import_handle =
            asset_server.load::<Shader>("shaders/custom_material_import.wesl");
        app.insert_resource(UtilityShader(utils_handle))
            .insert_resource(VertexAttrShader(vertex_attr_handle))
            .insert_resource(CustomMaterialImportShader(custom_material_import_handle));
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(CustomMaterial {
            time: Vec4::ZERO,
            party_mode: false,
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn update(
    time: Res<Time>,
    mut query: Query<(&MeshMaterial3d<CustomMaterial>, &mut Transform)>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (material, mut transform) in query.iter_mut() {
        let material = materials.get_mut(material).unwrap();
        material.time.x = time.elapsed_secs();
        if keys.just_pressed(KeyCode::Space) {
            material.party_mode = !material.party_mode;
        }

        if material.party_mode {
            transform.rotate(Quat::from_rotation_y(0.005));
        }
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(CustomMaterialKey)]
struct CustomMaterial {
    // Needed for 16 bit alignment in WebGL2
    #[uniform(0)]
    time: Vec4,
    party_mode: bool,
}

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct CustomMaterialKey {
    party_mode: bool,
}

impl From<&CustomMaterial> for CustomMaterialKey {
    fn from(material: &CustomMaterial) -> Self {
        Self {
            party_mode: material.party_mode,
        }
    }
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        FRAGMENT_SHADER_ASSET_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader_defs.push(ShaderDefVal::Bool(
            "PARTY_MODE".to_string(),
            key.bind_group_data.party_mode,
        ));
        Ok(())
    }
}
