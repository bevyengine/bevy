//! Demonstrates how to use `MeshPass`.
//!
//! TODO: Documentation

use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d},
    pbr::{
        DrawMesh, MainPass, MaterialPipelineSpecializer, MeshPass, MeshPassPlugin, PassShaders,
        SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup,
        SetMeshViewBindingArrayBindGroup, ShaderSet, MATERIAL_BIND_GROUP_INDEX,
    },
    prelude::*,
};
use bevy_render::{
    extract_component::ExtractComponent, render_phase::SetItemPipeline,
    render_resource::AsBindGroup,
};

const SHADER_ASSET_PATH: &str = "shaders/custom_mesh_pass_material.wgsl";
const OUTLINE_SHADER_ASSET_PATH: &str = "shaders/custom_mesh_pass_outline_material.wgsl";

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        // You can use `register_required_components` to add our `HullOutlinePass` to all cameras.
        // .register_required_components::<Camera3d, HullOutlinePass>()
        .add_plugins(MeshPassPlugin::<HullOutlinePass>::default())
        .add_systems(Startup, setup)
        .run();
}

type DrawMaterial2 = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<MATERIAL_BIND_GROUP_INDEX>,
    DrawMesh,
);

#[derive(Clone, Copy, Default, Component, ExtractComponent)]
struct HullOutlinePass;

impl MeshPass for HullOutlinePass {
    type ViewKeySource = MainPass;
    type Specializer = MaterialPipelineSpecializer;
    type PhaseItems = (Opaque3d, AlphaMask3d);
    type RenderCommand = DrawMaterial2;
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[uniform(1)]
    outline_color: LinearRgba,
}

impl Material for CustomMaterial {
    fn shaders() -> PassShaders {
        let mut pass_shaders = PassShaders::default();
        // Add HullOutlinePass shaders
        pass_shaders.insert(
            HullOutlinePass::id(),
            ShaderSet {
                vertex: OUTLINE_SHADER_ASSET_PATH.into(),
                fragment: OUTLINE_SHADER_ASSET_PATH.into(),
            },
        );
        // This won't work! We can't reuse the same `PhaseItem`s in one material.
        // pass_shaders.insert(
        //     MainPass::id(),
        //     ShaderSet {
        //         vertex: SHADER_ASSET_PATH.into(),
        //         fragment: SHADER_ASSET_PATH.into(),
        //     },
        // );
        pass_shaders
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // Spawn the cube.
    commands.spawn((
        MeshMaterial3d(materials.add(CustomMaterial {
            color: LinearRgba::new(0.8, 0.5, 0.6, 1.0),
            outline_color: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        })),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        // We are not using `register_required_components`, so let's manually
        // mark the camera for rendering the custom pass.
        HullOutlinePass,
    ));
}
