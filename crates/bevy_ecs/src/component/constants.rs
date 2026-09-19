//! Constant components included in every world.

use crate::component::ComponentId;

/// [`ComponentId`] of the [`Add`](crate::lifecycle::Add) component used in lifecycle observers.
pub const ADD: ComponentId = ComponentId::from_u32(0);
/// [`ComponentId`] of the [`Insert`](crate::lifecycle::Insert) component used in lifecycle observers.
pub const INSERT: ComponentId = ComponentId::from_u32(1);
/// [`ComponentId`] of the [`Discard`](crate::lifecycle::Discard) component used in lifecycle observers.
pub const DISCARD: ComponentId = ComponentId::from_u32(2);
/// [`ComponentId`] of the [`Remove`](crate::lifecycle::Remove) component used in lifecycle observers.
pub const REMOVE: ComponentId = ComponentId::from_u32(3);
/// [`ComponentId`] of the [`Despawn`](crate::lifecycle::Despawn) component used in lifecycle observers.
pub const DESPAWN: ComponentId = ComponentId::from_u32(4);
/// [`ComponentId`] of the [`IsResource`](crate::resource::IsResource) component used to mark entities with resources.
pub const IS_RESOURCE: ComponentId = ComponentId::from_u32(5);
/// [`ComponentId`] of the [`ArchetypeCreated`](crate::archetype::ArchetypeCreated) component used as an observer event key.
pub(crate) const ARCHETYPE_CREATED: ComponentId = ComponentId::from_u32(6);
