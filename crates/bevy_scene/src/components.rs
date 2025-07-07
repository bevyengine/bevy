use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use derive_more::derive::From;

#[cfg(feature = "bevy_render")]
use bevy_render::view::visibility::Visibility;

use crate::{DynamicScene, Scene};

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[cfg_attr(feature = "bevy_render", require(Visibility))]
pub struct SceneRoot(pub Handle<Scene>);

/// Adding this component will spawn the scene as a child of that entity.
/// Once it's spawned, the entity will have a [`SceneInstance`](crate::SceneInstance) component.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Transform)]
#[cfg_attr(feature = "bevy_render", require(Visibility))]
pub struct DynamicSceneRoot(pub Handle<DynamicScene>);
