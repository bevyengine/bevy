pub mod handle;
pub mod material;
pub mod material_data;
pub mod material_pipeline;

#[cfg(test)]
mod tests {
    use bevy_app::{App, Plugin};
    use bevy_reflect::Reflect;

    use crate::handle::MaterialHandle;
    use crate::material_pipeline::MaterialPipeline;

    #[derive(Reflect)]
    pub struct TestPipeline;

    type TestMaterial<M> = MaterialHandle<M, TestPipeline>;
}
