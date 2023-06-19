//! Types that enable reflection support.

use crate::entity::Entity;
use bevy_reflect::{
    impl_from_reflect_value, impl_reflect_value, ReflectDeserialize, ReflectSerialize,
};

mod component;
mod entity_commands;
mod map_entities;
mod resource;

pub use component::{ReflectComponent, ReflectComponentFns};
pub use entity_commands::EntityCommandsReflectExtension;
pub use map_entities::ReflectMapEntities;
pub use resource::{ReflectResource, ReflectResourceFns};

impl_reflect_value!((in bevy_ecs) Entity(Hash, PartialEq, Serialize, Deserialize));
impl_from_reflect_value!(Entity);
