//! Internal components used by bevy with a fixed component id.
//! Constants are used to skip [`TypeId`] lookups in hot paths.
use super::*;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// [`ComponentId`] for [`OnAdd`]
pub const ON_ADD: ComponentId = ComponentId::new(0);
/// [`ComponentId`] for [`OnInsert`]
pub const ON_INSERT: ComponentId = ComponentId::new(1);
/// [`ComponentId`] for [`OnReplace`]
pub const ON_REPLACE: ComponentId = ComponentId::new(2);
/// [`ComponentId`] for [`OnRemove`]
pub const ON_REMOVE: ComponentId = ComponentId::new(3);
/// [`ComponentId`] for [`OnDespawn`]
pub const ON_DESPAWN: ComponentId = ComponentId::new(4);

/// Trigger emitted when a component is inserted onto an entity that does not already have that
/// component. Runs before `OnInsert`.
/// See [`crate::component::ComponentHooks::on_add`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnAdd;

/// Trigger emitted when a component is inserted, regardless of whether or not the entity already
/// had that component. Runs after `OnAdd`, if it ran.
/// See [`crate::component::ComponentHooks::on_insert`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnInsert;

/// Trigger emitted when a component is inserted onto an entity that already has that component.
/// Runs before the value is replaced, so you can still access the original component data.
/// See [`crate::component::ComponentHooks::on_replace`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnReplace;

/// Trigger emitted when a component is removed from an entity, and runs before the component is
/// removed, so you can still access the component data.
/// See [`crate::component::ComponentHooks::on_remove`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnRemove;

/// Trigger emitted for each component on an entity when it is despawned.
/// See [`crate::component::ComponentHooks::on_despawn`] for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnDespawn;
