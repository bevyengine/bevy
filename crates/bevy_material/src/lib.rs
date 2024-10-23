pub mod component;
pub mod material;
pub mod material_pipeline;

pub mod prelude {
    pub use super::material::{Material, MaterialPlugin};
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, Plugin};
    use bevy_reflect::Reflect;

    use crate::component::MaterialComponent;
    use crate::material_pipeline::MaterialPipeline;
    use crate::material_trait_alias;

    #[derive(Reflect)]
    pub struct TestPipeline;

    type TestMeshMaterial<M> = MaterialComponent<M, TestPipeline>;

    material_trait_alias!(TestMaterial, TestPipeline);

    impl MaterialPipeline for TestPipeline {
        type MaterialProperties = ();
        type ShaderKey = ();
        type PipelineInfo<'a, M: TestMaterial> = ();

        fn material_plugin<M: TestMaterial>() -> impl Plugin {
            |_: &mut App| {}
        }
    }
}
