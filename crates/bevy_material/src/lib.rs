pub mod component;
pub mod material;
pub mod material_data;
pub mod material_pipeline;

pub mod prelude {
    pub use super::material::{Material, MaterialPlugin};
    pub use super::material_pipeline::*;
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, Plugin};
    use bevy_reflect::Reflect;

    use crate::component::MaterialComponent;
    use crate::material_pipeline::MaterialPipeline;
    use crate::prelude::Material;

    #[derive(Reflect)]
    pub struct TestPipeline;

    type TestMaterial<M> = MaterialComponent<M, TestPipeline>;
}
