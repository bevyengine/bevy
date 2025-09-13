use bevy_asset::{AsAssetId, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

use bevy_camera::visibility::Visibility;

use crate::{DynamicScene, Scene};

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct SceneRoot(pub Handle<Scene>);

impl AsAssetId for SceneRoot {
    type Asset = Scene;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct DynamicSceneRoot(pub Handle<DynamicScene>);

impl AsAssetId for DynamicSceneRoot {
    type Asset = DynamicScene;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}
