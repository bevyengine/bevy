//! Internal components used by bevy with a fixed component id.
//! Constants are used to skip [`TypeId`] lookups in hot paths.
use super::*;
use crate::{self as bevy_ecs};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// Trigger emitted when a component is added to an entity. See [`crate::component::ComponentHooks::on_add`]
/// for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnAdd;

/// Trigger emitted when a component is inserted onto an entity. See [`crate::component::ComponentHooks::on_insert`]
/// for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnInsert;

/// Trigger emitted when a component is replaced on an entity. See [`crate::component::ComponentHooks::on_replace`]
/// for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnReplace;

/// Trigger emitted when a component is removed from an entity. See [`crate::component::ComponentHooks::on_remove`]
/// for more information.
#[derive(Event, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Debug))]
pub struct OnRemove;
