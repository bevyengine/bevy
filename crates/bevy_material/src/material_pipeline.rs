use bevy_app::Plugin;
use bevy_ecs::system::{Res, SystemParam};
use bevy_reflect::TypePath;
use bevy_render::render_asset::RenderAssets;
use core::hash::Hash;

use crate::material::{
    Material, MaterialBindGroup, MaterialLayout, MaterialProperties, MaterialShaders,
};

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync + 'static;
    type ShaderKey: Hash + Eq + Send + Sync + 'static;
    type PipelineInfo<'a, M: Material<Self>>;

    fn material_plugin<M: Material<Self>>() -> impl Plugin;
}

#[derive(SystemParam)]
pub struct MaterialData<'w, M: Material<P>, P: MaterialPipeline> {
    pub layout: Res<'w, MaterialLayout<M>>,
    pub shaders: Res<'w, MaterialShaders<M, P>>,
    pub bind_group: Res<'w, RenderAssets<MaterialBindGroup<M>>>,
    pub properties: Res<'w, RenderAssets<MaterialProperties<M, P>>>,
}
