//! Contains the [`AutoFocus`] component and related machinery.

use bevy_ecs::{component::ComponentId, prelude::*, world::DeferredWorld};

use crate::{tab_navigation::TabIndex, SetInputFocus};

/// Indicates that this widget should automatically receive focus when it's added.
#[derive(Debug, Default, Component, Copy, Clone)]
#[component(on_add = on_auto_focus_added)]
pub struct AutoFocus;

fn on_auto_focus_added(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    if world.entity(entity).contains::<TabIndex>() {
        world.set_input_focus(entity);
    }
}
