use bevy_app::Plugin;
use bevy_ecs::system::{Res, SystemParam};
use bevy_reflect::TypePath;
use bevy_render::render_asset::RenderAssets;
use core::hash::Hash;

use crate::material::{
    Material, MaterialBindGroup, MaterialLayout, MaterialProperties, MaterialShaders,
    RenderMaterialInstances,
};

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync + 'static;
    type ShaderKey: Hash + Eq + Send + Sync + 'static;
    type PipelineInfo<'a, M: Material<Self>>;

    fn material_plugin<M: Material<Self>>() -> impl Plugin;
}
