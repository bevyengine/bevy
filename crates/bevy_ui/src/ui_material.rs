use crate::Node;
use bevy_asset::{Asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::ExtractComponent,
    render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef},
};
use bevy_ui_render::UiMaterial;
use core::hash::Hash;
use derive_more::derive::From;

#[derive(
    Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq, ExtractComponent, From,
)]
#[reflect(Component, Default)]
#[require(Node)]
pub struct MaterialNode<M: UiMaterial>(pub Handle<M>);

impl<M: UiMaterial> Default for MaterialNode<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: UiMaterial> From<MaterialNode<M>> for AssetId<M> {
    fn from(material: MaterialNode<M>) -> Self {
        material.id()
    }
}

impl<M: UiMaterial> From<&MaterialNode<M>> for AssetId<M> {
    fn from(material: &MaterialNode<M>) -> Self {
        material.id()
    }
}
