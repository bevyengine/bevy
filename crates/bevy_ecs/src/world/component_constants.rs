use super::*;
use crate as bevy_ecs;
/// Internal components used by bevy with a fixed component id.
/// Constants are used to skip [`TypeId`] lookups in hot paths.

/// [`ComponentId`] for [`OnAdd`]
pub const ON_ADD: ComponentId = ComponentId::new(0);
/// [`ComponentId`] for [`OnInsert`]
pub const ON_INSERT: ComponentId = ComponentId::new(1);
/// [`ComponentId`] for [`OnRemove`]
pub const ON_REMOVE: ComponentId = ComponentId::new(2);

/// Event emitted when a component is added to an entity.
#[derive(Component)]
pub struct OnAdd;

/// Event emitted when a component is inserted on to to an entity.
#[derive(Component)]
pub struct OnInsert;

/// Event emitted when a component is removed from an entity.
#[derive(Component)]
pub struct OnRemove;
