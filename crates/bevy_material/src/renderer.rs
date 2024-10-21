use bevy_app::App;
use bevy_asset::AssetId;
use bevy_ecs::{
    component::Component,
    system::{SystemParam, SystemParamItem},
};
use bevy_reflect::{GetTypeRegistration, TypePath};

use crate::material::Material;

pub enum QueueError {}

pub trait Renderer: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync + 'static;
    type ShaderKey;
    type PipelineInfo<'a, M: Material<Self>>;
    type QueueParam: SystemParam;
    type SourceComponent<M: Material<Self>>: Component + GetTypeRegistration;

    fn source_asset_id<M: Material<Self>>(source: &Self::SourceComponent<M>) -> AssetId<M>;

    fn material_plugin<M: Material<Self>>(app: &mut App);

    fn queue_one<M: Material<Self>>(
        material: &M::Data,
        queuing_info: &Self::MaterialProperties,
        param: &mut SystemParamItem<Self::QueueParam>,
    ) -> Result<(), QueueError>;
}
