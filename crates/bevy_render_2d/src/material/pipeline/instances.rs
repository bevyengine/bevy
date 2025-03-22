use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;
use bevy_render::sync_world::MainEntityHashMap;

use crate::material::Material2d;

#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterial2dInstances<M: Material2d>(MainEntityHashMap<AssetId<M>>);

impl<M: Material2d> Default for RenderMaterial2dInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}
