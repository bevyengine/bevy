use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

use bevy_camera::visibility::Visibility;

use crate::{DynamicScene, Scene};

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct SceneRoot(pub Handle<Scene>);

impl Default for SceneRoot {
    fn default() -> Self {
        Self(Handle::default())
    }
}

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[require(Visibility)]
pub struct DynamicSceneRoot(pub Handle<DynamicScene>);

impl Default for DynamicSceneRoot {
    fn default() -> Self {
        Self(Handle::default())
    }
}
