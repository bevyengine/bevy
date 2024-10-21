pub mod material;
pub mod renderer;

pub mod prelude {
    pub use super::material::{Material, MaterialPlugin};
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_reflect::Reflect;

    use crate::{prelude::Material, renderer::Renderer};

    #[derive(Reflect)]
    pub struct TestRenderer;

    impl Renderer for TestRenderer {
        type MaterialProperties = ();
        type ShaderKey = ();
        type PipelineInfo<'a, M: Material<Self>> = ();
        type QueueParam = ();
        type SourceComponent<M: Material<Self>> = ();

        fn source_asset_id<M: Material<Self>>(
            source: &Self::SourceComponent<M>,
        ) -> bevy_asset::AssetId<M> {
            todo!()
        }

        fn material_plugin<M: Material<Self>>(app: &mut App) {}

        fn queue_one<M: Material<Self>>(
            material: &M::Data,
            queuing_info: &Self::MaterialProperties,
            param: &mut bevy_ecs::system::SystemParamItem<Self::QueueParam>,
        ) -> Result<(), crate::renderer::QueueError> {
            todo!()
        }
    }
}
