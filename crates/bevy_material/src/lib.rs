pub mod component;
pub mod material;
pub mod renderer;

pub mod prelude {
    pub use super::material::{Material, MaterialPlugin};
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_reflect::Reflect;

    use crate::component::MaterialComponent;
    use crate::material_trait_alias;
    use crate::renderer::MaterialPipeline;

    #[derive(Reflect)]
    pub struct TestPipeline;

    type TestMeshMaterial<M> = MaterialComponent<M, TestPipeline>;

    material_trait_alias!(TestMaterial, TestPipeline);

    impl MaterialPipeline for TestPipeline {
        type MaterialProperties = ();
        type ShaderKey = ();
        type PipelineInfo<'a, M: TestMaterial> = ();

        fn material_plugin<M: TestMaterial>(app: &mut App) {}
    }
}
