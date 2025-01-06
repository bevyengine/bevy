//! Contains the [`AutoFocus`] component and related machinery.

use bevy_ecs::{component::ComponentId, prelude::*, world::DeferredWorld};

use crate::InputFocus;

/// Indicates that this widget should automatically receive [`InputFocus`].
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
    if let Some(mut input_focus) = world.get_resource_mut::<InputFocus>() {
        input_focus.set(entity);
    }
}
