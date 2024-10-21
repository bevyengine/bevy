use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, SystemParamItem},
    world::{FromWorld, World},
};
use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId, AssetPath};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    render_resource::{AsBindGroup, AsBindGroupError, BindGroupLayout, PreparedBindGroup},
    renderer::RenderDevice,
    sync_world::{MainEntity, MainEntityHashMap},
    view::ViewVisibility,
    Extract, ExtractSchedule, RenderApp,
};

use crate::renderer::Renderer;

pub enum SpecializeMaterialPipelineError {}

pub trait BaseMaterial: Asset + AsBindGroup + Clone + Sized {}

impl<T: Asset + AsBindGroup + Clone + Sized> BaseMaterial for T {}

pub trait Material<R: Renderer>: BaseMaterial {
    fn properties(&self) -> R::MaterialProperties;
    fn shaders(key: R::ShaderKey) -> Option<AssetPath<'static>>;
    fn specialize(info: R::PipelineInfo<'_, Self>) -> Result<(), SpecializeMaterialPipelineError>;
}

pub struct MaterialPlugin<M: Material<R>, R: Renderer>(PhantomData<fn(M, R)>);

impl<M: Material<R>, R: Renderer> Plugin for MaterialPlugin<M, R> {
    fn build(&self, app: &mut App) {
    }

    fn finish(&self, app: &mut App) {
    }
}
