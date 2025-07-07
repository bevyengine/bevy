//! Contains the [`AutoFocus`] component and related machinery.

use bevy_ecs::{lifecycle::HookContext, prelude::*, world::DeferredWorld};

use crate::InputFocus;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};

/// Indicates that this widget should automatically receive [`InputFocus`].
///
/// This can be useful for things like dialog boxes, the first text input in a form,
/// or the first button in a game menu.
///
/// The focus is swapped when this component is added
/// or an entity with this component is spawned.
#[derive(Debug, Default, Component, Copy, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Component, Clone)
)]
#[component(on_add = on_auto_focus_added)]
pub struct AutoFocus;

fn on_auto_focus_added(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(mut input_focus) = world.get_resource_mut::<InputFocus>() {
        input_focus.set(entity);
    }
}
