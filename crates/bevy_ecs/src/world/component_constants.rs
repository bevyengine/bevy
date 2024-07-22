use super::*;
use crate::{self as bevy_ecs};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
/// Internal components used by bevy with a fixed component id.
/// Constants are used to skip [`TypeId`] lookups in hot paths.

/// [`ComponentId`] for [`OnAdd`]
pub const ON_ADD: ComponentId = ComponentId::new(0);
/// [`ComponentId`] for [`OnInsert`]
pub const ON_INSERT: ComponentId = ComponentId::new(1);
/// [`ComponentId`] for [`OnRemove`]
pub const ON_REMOVE: ComponentId = ComponentId::new(2);

/// Trigger emitted when a component is added to an entity.
#[derive(Event)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct OnAdd;

/// Trigger emitted when a component is inserted on to to an entity.
#[derive(Event)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct OnInsert;

/// Trigger emitted when a component is removed from an entity.
#[derive(Event)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct OnRemove;
