use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

#[cfg(feature = "bevy_render")]
use bevy_render::view::visibility::Visibility;

use crate::{DynamicScene, Scene};

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[require(Transform)]
#[cfg_attr(feature = "bevy_render", require(Visibility))]
pub struct SceneRoot(pub Handle<Scene>);

impl From<Handle<Scene>> for SceneRoot {
    fn from(handle: Handle<Scene>) -> Self {
        Self(handle)
    }
}

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[require(Transform)]
#[cfg_attr(feature = "bevy_render", require(Visibility))]
pub struct DynamicSceneRoot(pub Handle<DynamicScene>);

impl From<Handle<DynamicScene>> for DynamicSceneRoot {
    fn from(handle: Handle<DynamicScene>) -> Self {
        Self(handle)
    }
}
