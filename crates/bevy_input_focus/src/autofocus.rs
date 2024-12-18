//! Contains the [`AutoFocus`] component and related machinery.

use bevy_ecs::{component::ComponentId, prelude::*, world::DeferredWorld};

use crate::SetInputFocus;

/// Indicates that this widget should automatically receive [`InputFocus`](crate::InputFocus).
///
/// This can be useful for things like dialog boxes, the first text input in a form,
/// or the first button in a game menu.
///
/// The focus is swapped when this component is added
/// or an entity with this component is spawned.
#[derive(Debug, Default, Component, Copy, Clone)]
#[component(on_add = on_auto_focus_added)]
pub struct AutoFocus;

fn on_auto_focus_added(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    world.set_input_focus(entity);
}
