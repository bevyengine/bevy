use super::*;
use crate as bevy_ecs;

/// [`ComponentId`] for [`Any`]
pub const ANY: ComponentId = ComponentId::new(0);
/// [`ComponentId`] for [`NoEvent`]
pub const NO_EVENT: ComponentId = ComponentId::new(1);
/// [`ComponentId`] for [`OnAdd`]
pub const ON_ADD: ComponentId = ComponentId::new(2);
/// [`ComponentId`] for [`OnInsert`]
pub const ON_INSERT: ComponentId = ComponentId::new(3);
/// [`ComponentId`] for [`OnRemove`]
pub const ON_REMOVE: ComponentId = ComponentId::new(4);

/// Event emitted when a component is added to an entity.
#[derive(Component)]
pub struct OnAdd;

/// Event emitted when a component is inserted on to to an entity.
#[derive(Component)]
pub struct OnInsert;

/// Event emitted when a component is removed from an entity.
#[derive(Component)]
pub struct OnRemove;

/// Type used to signify observers that are listening to multiple events
/// so cannot access event data.
#[derive(Component)]
pub struct NoEvent;

/// Type used to signify observers that listen to events targetting any entities or components.
#[derive(Component)]
pub struct Any;
