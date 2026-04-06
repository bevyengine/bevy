use bevy_asset::{AsAssetId, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent, template::FromTemplate};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

use bevy_camera::visibility::Visibility;

use crate::{DynamicWorld, WorldAsset};

/// Adding this component will spawn the world as a child of that entity.
/// Once it's spawned, the entity will have a [`WorldInstance`](crate::WorldInstance) component.
///
/// Note: This was recently renamed from `WorldAssetRoot`, in the interest of giving "scene" terminology to
/// Bevy's next generation scene system, available in `bevy_scene`.
#[derive(
    Component, FromTemplate, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From,
)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct WorldAssetRoot(pub Handle<WorldAsset>);

impl AsAssetId for WorldAssetRoot {
    type Asset = WorldAsset;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// Adding this component will spawn the world as a child of that entity.
/// Once it's spawned, the entity will have a [`WorldInstance`](crate::WorldInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct DynamicWorldRoot(pub Handle<DynamicWorld>);

impl AsAssetId for DynamicWorldRoot {
    type Asset = DynamicWorld;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}
