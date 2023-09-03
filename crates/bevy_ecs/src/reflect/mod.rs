//! Types that enable reflection support.

use crate::entity::Entity;
use bevy_reflect::{impl_reflect_value, ReflectDeserialize, ReflectSerialize};

mod bundle;
mod component;
mod entity_commands;
mod map_entities;
mod registry;
mod resource;

pub use bundle::{ReflectBundle, ReflectBundleFns};
pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::ReflectCommandExt;
pub use map_entities::ReflectMapEntities;
pub use registry::{AppTypeRegistry, ReadTypeRegistry};
pub use resource::{ReflectResource, ReflectResourceFns};

impl_reflect_value!((in bevy_ecs) Entity(Hash, PartialEq, Serialize, Deserialize));
