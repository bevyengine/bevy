//! Types that enable reflection support.

use std::ops::{Deref, DerefMut};

use crate as bevy_ecs;
use crate::{entity::Entity, system::Resource};
use bevy_reflect::{impl_reflect_value, ReflectDeserialize, ReflectSerialize, TypeRegistryArc};

mod bundle;
mod component;
mod entity_commands;
mod map_entities;
mod resource;

pub use bundle::{ReflectBundle, ReflectBundleFns};
pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::ReflectCommandExt;
pub use map_entities::ReflectMapEntities;
pub use resource::{ReflectResource, ReflectResourceFns};

/// A [`Resource`] storing [`TypeRegistry`](bevy_reflect::TypeRegistry) for
/// type registrations relevant to a whole app.
#[derive(Resource, Clone, Default)]
pub struct AppTypeRegistry(pub TypeRegistryArc);

impl Deref for AppTypeRegistry {
    type Target = TypeRegistryArc;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppTypeRegistry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl_reflect_value!((in bevy_ecs) Entity(Hash, PartialEq, Serialize, Deserialize));
