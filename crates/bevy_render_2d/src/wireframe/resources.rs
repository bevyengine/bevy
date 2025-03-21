use bevy_asset::{AssetId, Handle};
use bevy_color::Color;
use bevy_ecs::{reflect::ReflectResource, resource::Resource};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::extract_resource::ExtractResource;

use super::Wireframe2dMaterial;

/// Global configuration for 2d wireframes
#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct Wireframe2dConfig {
    /// Whether to show wireframes for all 2D meshes.
    /// Can be overridden for individual meshes by adding a [`Wireframe2d`] or [`NoWireframe2d`] component.
    pub global: bool,
    /// If [`Self::global`] is set, any [`Entity`] that does not have a [`Wireframe2d`] component attached to it will have
    /// wireframes using this color. Otherwise, this will be the fallback color for any entity that has a [`Wireframe2d`],
    /// but no [`Wireframe2dColor`].
    pub default_color: Color,
}

#[derive(Resource)]
pub struct GlobalWireframe2dMaterial {
    // This handle will be reused when the global config is enabled
    pub handle: Handle<Wireframe2dMaterial>,
}

impl GlobalWireframe2dMaterial {
    pub fn handle(&self) -> Handle<Wireframe2dMaterial> {
        self.handle.clone()
    }
}

impl From<&GlobalWireframe2dMaterial> for AssetId<Wireframe2dMaterial> {
    fn from(value: &GlobalWireframe2dMaterial) -> Self {
        value.handle.id()
    }
}
