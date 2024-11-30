#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Keyboard focus system for Bevy.
//!
//! This crate provides a system for managing keyboard focus in Bevy applications, including:
//! * A resource for tracking which entity has keyboard focus.
//! * Methods for getting and setting keyboard focus.
//! * Event definitions for triggering bubble-able keyboard input events to the focused entity.
//! * A system for dispatching keyboard input events to the focused entity.
//!
//! This crate does *not* provide any integration with UI widgets, or provide functions for
//! tab navigation or gamepad-based focus navigation, as those are typically application-specific.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventReader},
    query::With,
    system::{Commands, Query, Res, Resource},
    world::{Command, DeferredWorld, World},
};
use bevy_hierarchy::Parent;
use bevy_input::keyboard::KeyboardInput;
use bevy_window::PrimaryWindow;

/// Resource representing which entity has keyboard focus, if any. Keyboard events will be
/// dispatched to the current focus entity, or to the primary window if no entity has focus.
#[derive(Clone, Debug, Resource)]
pub struct KeyboardFocus(pub Option<Entity>);

/// Resource representing whether the keyboard focus indicator should be visible. It's up to the
/// current focus navigation system to set this resource. For a desktop/web style of user interface
/// this would be set to true when the user presses the tab key, and set to false when the user
/// clicks on a different element.
#[derive(Clone, Debug, Resource)]
pub struct KeyboardFocusVisible(pub bool);

/// Helper functions for [`World`] and [`DeferredWorld`] to set and clear keyboard focus.
pub trait SetKeyboardFocus {
    /// Set keyboard focus to the given entity.
    fn set_keyboard_focus(&mut self, entity: Entity);
    /// Clear keyboard focus.
    fn clear_keyboard_focus(&mut self);
}

impl SetKeyboardFocus for World {
    fn set_keyboard_focus(&mut self, entity: Entity) {
        if let Some(mut focus) = self.get_resource_mut::<KeyboardFocus>() {
            focus.0 = Some(entity);
        }
    }

    fn clear_keyboard_focus(&mut self) {
        if let Some(mut focus) = self.get_resource_mut::<KeyboardFocus>() {
            focus.0 = None;
        }
    }
}

impl<'w> SetKeyboardFocus for DeferredWorld<'w> {
    fn set_keyboard_focus(&mut self, entity: Entity) {
        if let Some(mut focus) = self.get_resource_mut::<KeyboardFocus>() {
            focus.0 = Some(entity);
        }
    }

    fn clear_keyboard_focus(&mut self) {
        if let Some(mut focus) = self.get_resource_mut::<KeyboardFocus>() {
            focus.0 = None;
        }
    }
}

/// Command to set keyboard focus to the given entity.
pub struct SetFocusCommand(Option<Entity>);

impl Command for SetFocusCommand {
    fn apply(self, world: &mut World) {
        if let Some(mut focus) = world.get_resource_mut::<KeyboardFocus>() {
            focus.0 = self.0;
        }
    }
}

/// A bubble-able event for keyboard input. This event is normally dispatched to the current
/// keyboard focus entity, if any. If no entity has keyboard focus, then the event is dispatched to
/// the main window.
#[derive(Clone, Debug, Component)]
pub struct FocusKeyboardInput(pub KeyboardInput);

impl Event for FocusKeyboardInput {
    type Traversal = &'static Parent;

    const AUTO_PROPAGATE: bool = true;
}

/// Plugin which registers the system for dispatching keyboard events based on focus and
/// hover state.
pub struct InputDispatchPlugin;

impl Plugin for InputDispatchPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KeyboardFocus(None))
            .add_systems(Update, dispatch_keyboard_input);
    }
}

/// System whcich dispatches keyboard input events to the focused entity, or to the primary window
/// if no entity has focus.
fn dispatch_keyboard_input(
    mut key_events: EventReader<KeyboardInput>,
    focus: Res<KeyboardFocus>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    // If an element has keyboard focus, then dispatch the key event to that element.
    if let Some(focus_elt) = focus.0 {
        for ev in key_events.read() {
            commands.trigger_targets(FocusKeyboardInput(ev.clone()), focus_elt);
        }
    } else {
        // If no element has keyboard focus, then dispatch the key event to the primary window.
        // There should be only one primary window.
        if let Ok(window) = windows.get_single() {
            for ev in key_events.read() {
                commands.trigger_targets(FocusKeyboardInput(ev.clone()), window);
            }
        }
    }
}

/// Trait which defines methods to check if an entity currently has focus. This is implemented
/// for both [`World`] and [`DeferredWorld`].
pub trait IsFocused {
    /// Returns true if the given entity has keyboard focus.
    fn is_focused(&self, entity: Entity) -> bool;

    /// Returns true if the given entity or any of its descendants has keyboard focus.
    fn is_focus_within(&self, entity: Entity) -> bool;

    /// Returns true if the given entity has keyboard focus and the focus indicator is visible.
    fn is_focus_visible(&self, entity: Entity) -> bool;

    /// Returns true if the given entity, or any descenant, has keyboard focus and the focus
    /// indicator is visible.
    fn is_focus_within_visible(&self, entity: Entity) -> bool;
}

impl IsFocused for DeferredWorld<'_> {
    fn is_focused(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocus>()
            .map(|f| f.0)
            .unwrap_or_default()
            .map(|f| f == entity)
            .unwrap_or_default()
    }

    fn is_focus_within(&self, entity: Entity) -> bool {
        let Some(focus_resource) = self.get_resource::<KeyboardFocus>() else {
            return false;
        };
        let Some(focus) = focus_resource.0 else {
            return false;
        };
        let mut e = entity;
        loop {
            if e == focus {
                return true;
            }
            if let Some(parent) = self.entity(e).get::<Parent>() {
                e = parent.get();
            } else {
                break;
            }
        }
        false
    }

    fn is_focus_visible(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocusVisible>()
            .map(|vis| vis.0)
            .unwrap_or_default()
            && self.is_focused(entity)
    }

    fn is_focus_within_visible(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocusVisible>()
            .map(|vis| vis.0)
            .unwrap_or_default()
            && self.is_focus_within(entity)
    }
}

impl IsFocused for World {
    fn is_focused(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocus>()
            .map(|f| f.0)
            .unwrap_or_default()
            .map(|f| f == entity)
            .unwrap_or_default()
    }

    fn is_focus_within(&self, entity: Entity) -> bool {
        let Some(focus_resource) = self.get_resource::<KeyboardFocus>() else {
            return false;
        };
        let Some(focus) = focus_resource.0 else {
            return false;
        };
        let mut e = entity;
        loop {
            if e == focus {
                return true;
            }
            if let Some(parent) = self.entity(e).get::<Parent>() {
                e = parent.get();
            } else {
                break;
            }
        }
        false
    }

    fn is_focus_visible(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocusVisible>()
            .map(|vis| vis.0)
            .unwrap_or_default()
            && self.is_focused(entity)
    }

    fn is_focus_within_visible(&self, entity: Entity) -> bool {
        self.get_resource::<KeyboardFocusVisible>()
            .map(|vis| vis.0)
            .unwrap_or_default()
            && self.is_focus_within(entity)
    }
}
