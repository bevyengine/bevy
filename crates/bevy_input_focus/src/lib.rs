#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! A UI-centric focus system for Bevy.
//!
//! This crate provides a system for managing input focus in Bevy applications, including:
//! * [`InputFocus`], a resource for tracking which entity has input focus.
//! * Methods for getting and setting input focus via [`InputFocus`] and [`IsFocusedHelper`].
//! * A generic [`FocusedInput`] event for input events which bubble up from the focused entity.
//! * Various navigation frameworks for moving input focus between entities based on user input, such as [`tab_navigation`] and [`directional_navigation`].
//!
//! This crate does *not* provide any integration with UI widgets: this is the responsibility of the widget crate,
//! which should depend on [`bevy_input_focus`](crate).

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod directional_navigation;
pub mod tab_navigation;

// This module is too small / specific to be exported by the crate,
// but it's nice to have it separate for code organization.
mod autofocus;
pub use autofocus::*;

use bevy_app::{App, Plugin, PostStartup, PreUpdate};
use bevy_ecs::{
    entity::Entities, prelude::*, query::QueryData, system::SystemParam, traversal::Traversal,
};
use bevy_input::{gamepad::GamepadButtonChangedEvent, keyboard::KeyboardInput, mouse::MouseWheel};
use bevy_window::{PrimaryWindow, Window};
use core::fmt::Debug;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};

/// Resource representing which entity has input focus, if any. Input events (other than pointer-like inputs) will be
/// dispatched to the current focus entity, or to the primary window if no entity has focus.
///
/// Changing the input focus is as easy as modifying this resource.
///
/// # Examples
///
/// From within a system:
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_input_focus::InputFocus;
///
/// fn clear_focus(mut input_focus: ResMut<InputFocus>) {
///   input_focus.clear();
/// }
/// ```
///
/// With exclusive (or deferred) world access:
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_input_focus::InputFocus;
///
/// fn set_focus_from_world(world: &mut World) {
///     let entity = world.spawn_empty().id();
///
///     // Fetch the resource from the world
///     let mut input_focus = world.resource_mut::<InputFocus>();
///     // Then mutate it!
///     input_focus.set(entity);
///
///     // Or you can just insert a fresh copy of the resource
///     // which will overwrite the existing one.
///     world.insert_resource(InputFocus::from_entity(entity));
/// }
/// ```
#[derive(Clone, Debug, Default, Resource)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Resource, Clone)
)]
pub struct InputFocus(pub Option<Entity>);

impl InputFocus {
    /// Create a new [`InputFocus`] resource with the given entity.
    ///
    /// This is mostly useful for tests.
    pub const fn from_entity(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// Set the entity with input focus.
    pub const fn set(&mut self, entity: Entity) {
        self.0 = Some(entity);
    }

    /// Returns the entity with input focus, if any.
    pub const fn get(&self) -> Option<Entity> {
        self.0
    }

    /// Clears input focus.
    pub const fn clear(&mut self) {
        self.0 = None;
    }
}

/// Resource representing whether the input focus indicator should be visible on UI elements.
///
/// Note that this resource is not used by [`bevy_input_focus`](crate) itself, but is provided for
/// convenience to UI widgets or frameworks that want to display a focus indicator.
/// [`InputFocus`] may still be `Some` even if the focus indicator is not visible.
///
/// The value of this resource should be set by your focus navigation solution.
/// For a desktop/web style of user interface this would be set to true when the user presses the tab key,
/// and set to false when the user clicks on a different element.
/// By contrast, a console-style UI intended to be navigated with a gamepad may always have the focus indicator visible.
///
/// To easily access information about whether focus indicators should be shown for a given entity, use the [`IsFocused`] trait.
///
/// By default, this resource is set to `false`.
#[derive(Clone, Debug, Resource, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Resource, Clone)
)]
pub struct InputFocusVisible(pub bool);

/// A bubble-able user input event that starts at the currently focused entity.
///
/// This event is normally dispatched to the current input focus entity, if any.
/// If no entity has input focus, then the event is dispatched to the main window.
///
/// To set up your own bubbling input event, add the [`dispatch_focused_input::<MyEvent>`](dispatch_focused_input) system to your app,
/// in the [`InputFocusSystems::Dispatch`] system set during [`PreUpdate`].
#[derive(EntityEvent, Clone, Debug, Component)]
#[entity_event(propagate = WindowTraversal, auto_propagate)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct FocusedInput<M: Message + Clone> {
    /// The entity that has received focused input.
    #[event_target]
    pub focused_entity: Entity,
    /// The underlying input message.
    pub input: M,
    /// The primary window entity.
    window: Entity,
}

/// An event which is used to set input focus. Trigger this on an entity, and it will bubble
/// until it finds a focusable entity, and then set focus to it.
#[derive(Clone, EntityEvent)]
#[entity_event(propagate = WindowTraversal, auto_propagate)]
pub struct AcquireFocus {
    /// The entity that has acquired focus.
    #[event_target]
    pub focused_entity: Entity,
    /// The primary window entity.
    window: Entity,
}

#[derive(QueryData)]
/// These are for accessing components defined on the targeted entity
pub struct WindowTraversal {
    child_of: Option<&'static ChildOf>,
    window: Option<&'static Window>,
}

impl<M: Message + Clone> Traversal<FocusedInput<M>> for WindowTraversal {
    fn traverse(item: Self::Item<'_, '_>, event: &FocusedInput<M>) -> Option<Entity> {
        let WindowTraversalItem { child_of, window } = item;

        // Send event to parent, if it has one.
        if let Some(child_of) = child_of {
            return Some(child_of.parent());
        };

        // Otherwise, send it to the window entity (unless this is a window entity).
        if window.is_none() {
            return Some(event.window);
        }

        None
    }
}

impl Traversal<AcquireFocus> for WindowTraversal {
    fn traverse(item: Self::Item<'_, '_>, event: &AcquireFocus) -> Option<Entity> {
        let WindowTraversalItem { child_of, window } = item;

        // Send event to parent, if it has one.
        if let Some(child_of) = child_of {
            return Some(child_of.parent());
        };

        // Otherwise, send it to the window entity (unless this is a window entity).
        if window.is_none() {
            return Some(event.window);
        }

        None
    }
}

/// Plugin which sets up systems for dispatching bubbling keyboard and gamepad button events to the focused entity.
///
/// To add bubbling to your own input events, add the [`dispatch_focused_input::<MyEvent>`](dispatch_focused_input) system to your app,
/// as described in the docs for [`FocusedInput`].
pub struct InputDispatchPlugin;

impl Plugin for InputDispatchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, set_initial_focus)
            .init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .add_systems(
                PreUpdate,
                (
                    dispatch_focused_input::<KeyboardInput>,
                    dispatch_focused_input::<GamepadButtonChangedEvent>,
                    dispatch_focused_input::<MouseWheel>,
                )
                    .in_set(InputFocusSystems::Dispatch),
            );
    }
}

/// System sets for [`bevy_input_focus`](crate).
///
/// These systems run in the [`PreUpdate`] schedule.
#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
pub enum InputFocusSystems {
    /// System which dispatches bubbled input events to the focused entity, or to the primary window.
    Dispatch,
}

/// Deprecated alias for [`InputFocusSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `InputFocusSystems`.")]
pub type InputFocusSet = InputFocusSystems;

/// If no entity is focused, sets the focus to the primary window, if any.
pub fn set_initial_focus(
    mut input_focus: ResMut<InputFocus>,
    window: Single<Entity, With<PrimaryWindow>>,
) {
    if input_focus.0.is_none() {
        input_focus.0 = Some(*window);
    }
}

/// System which dispatches bubbled input events to the focused entity, or to the primary window
/// if no entity has focus.
///
/// If the currently focused entity no longer exists (has been despawned), this system will
/// automatically clear the focus and dispatch events to the primary window instead.
pub fn dispatch_focused_input<M: Message + Clone>(
    mut input_reader: MessageReader<M>,
    mut focus: ResMut<InputFocus>,
    windows: Query<Entity, With<PrimaryWindow>>,
    entities: &Entities,
    mut commands: Commands,
) {
    if let Ok(window) = windows.single() {
        // If an element has keyboard focus, then dispatch the input event to that element.
        if let Some(focused_entity) = focus.0 {
            // Check if the focused entity is still alive
            if entities.contains(focused_entity) {
                for ev in input_reader.read() {
                    commands.trigger(FocusedInput {
                        focused_entity,
                        input: ev.clone(),
                        window,
                    });
                }
            } else {
                // If the focused entity no longer exists, clear focus and dispatch to window
                focus.0 = None;
                for ev in input_reader.read() {
                    commands.trigger(FocusedInput {
                        focused_entity: window,
                        input: ev.clone(),
                        window,
                    });
                }
            }
        } else {
            // If no element has input focus, then dispatch the input event to the primary window.
            // There should be only one primary window.
            for ev in input_reader.read() {
                commands.trigger(FocusedInput {
                    focused_entity: window,
                    input: ev.clone(),
                    window,
                });
            }
        }
    }
}

/// Trait which defines methods to check if an entity currently has focus.
///
/// This is implemented for [`World`] and [`IsFocusedHelper`].
/// [`DeferredWorld`](bevy_ecs::world::DeferredWorld) indirectly implements it through [`Deref`].
///
/// For use within systems, use [`IsFocusedHelper`].
///
/// Modify the [`InputFocus`] resource to change the focused entity.
///
/// [`Deref`]: std::ops::Deref
pub trait IsFocused {
    /// Returns true if the given entity has input focus.
    fn is_focused(&self, entity: Entity) -> bool;

    /// Returns true if the given entity or any of its descendants has input focus.
    ///
    /// Note that for unusual layouts, the focus may not be within the entity's visual bounds.
    fn is_focus_within(&self, entity: Entity) -> bool;

    /// Returns true if the given entity has input focus and the focus indicator should be visible.
    fn is_focus_visible(&self, entity: Entity) -> bool;

    /// Returns true if the given entity, or any descendant, has input focus and the focus
    /// indicator should be visible.
    fn is_focus_within_visible(&self, entity: Entity) -> bool;
}

/// A system param that helps get information about the current focused entity.
///
/// When working with the entire [`World`], consider using the [`IsFocused`] instead.
#[derive(SystemParam)]
pub struct IsFocusedHelper<'w, 's> {
    parent_query: Query<'w, 's, &'static ChildOf>,
    input_focus: Option<Res<'w, InputFocus>>,
    input_focus_visible: Option<Res<'w, InputFocusVisible>>,
}

impl IsFocused for IsFocusedHelper<'_, '_> {
    fn is_focused(&self, entity: Entity) -> bool {
        self.input_focus
            .as_deref()
            .and_then(|f| f.0)
            .is_some_and(|e| e == entity)
    }

    fn is_focus_within(&self, entity: Entity) -> bool {
        let Some(focus) = self.input_focus.as_deref().and_then(|f| f.0) else {
            return false;
        };
        if focus == entity {
            return true;
        }
        self.parent_query.iter_ancestors(focus).any(|e| e == entity)
    }

    fn is_focus_visible(&self, entity: Entity) -> bool {
        self.input_focus_visible.as_deref().is_some_and(|vis| vis.0) && self.is_focused(entity)
    }

    fn is_focus_within_visible(&self, entity: Entity) -> bool {
        self.input_focus_visible.as_deref().is_some_and(|vis| vis.0) && self.is_focus_within(entity)
    }
}

impl IsFocused for World {
    fn is_focused(&self, entity: Entity) -> bool {
        self.get_resource::<InputFocus>()
            .and_then(|f| f.0)
            .is_some_and(|f| f == entity)
    }

    fn is_focus_within(&self, entity: Entity) -> bool {
        let Some(focus) = self.get_resource::<InputFocus>().and_then(|f| f.0) else {
            return false;
        };
        let mut e = focus;
        loop {
            if e == entity {
                return true;
            }
            if let Some(parent) = self.entity(e).get::<ChildOf>().map(ChildOf::parent) {
                e = parent;
            } else {
                return false;
            }
        }
    }

    fn is_focus_visible(&self, entity: Entity) -> bool {
        self.get_resource::<InputFocusVisible>()
            .is_some_and(|vis| vis.0)
            && self.is_focused(entity)
    }

    fn is_focus_within_visible(&self, entity: Entity) -> bool {
        self.get_resource::<InputFocusVisible>()
            .is_some_and(|vis| vis.0)
            && self.is_focus_within(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::string::String;
    use bevy_app::Startup;
    use bevy_ecs::{observer::On, system::RunSystemOnce, world::DeferredWorld};
    use bevy_input::{
        keyboard::{Key, KeyCode},
        ButtonState, InputPlugin,
    };

    #[derive(Component, Default)]
    struct GatherKeyboardEvents(String);

    fn gather_keyboard_events(
        event: On<FocusedInput<KeyboardInput>>,
        mut query: Query<&mut GatherKeyboardEvents>,
    ) {
        if let Ok(mut gather) = query.get_mut(event.focused_entity) {
            if let Key::Character(c) = &event.input.logical_key {
                gather.0.push_str(c.as_str());
            }
        }
    }

    fn key_a_message() -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character("A".into()),
            state: ButtonState::Pressed,
            text: Some("A".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn test_no_panics_if_resource_missing() {
        let mut app = App::new();
        // Note that we do not insert InputFocus here!

        let entity = app.world_mut().spawn_empty().id();

        assert!(!app.world().is_focused(entity));

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| {
                assert!(!helper.is_focused(entity));
                assert!(!helper.is_focus_within(entity));
                assert!(!helper.is_focus_visible(entity));
                assert!(!helper.is_focus_within_visible(entity));
            })
            .unwrap();

        app.world_mut()
            .run_system_once(move |world: DeferredWorld| {
                assert!(!world.is_focused(entity));
                assert!(!world.is_focus_within(entity));
                assert!(!world.is_focus_visible(entity));
                assert!(!world.is_focus_within_visible(entity));
            })
            .unwrap();
    }

    #[test]
    fn initial_focus_unset_if_no_primary_window() {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputDispatchPlugin));

        app.update();

        assert_eq!(app.world().resource::<InputFocus>().0, None);
    }

    #[test]
    fn initial_focus_set_to_primary_window() {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputDispatchPlugin));

        let entity_window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();

        assert_eq!(app.world().resource::<InputFocus>().0, Some(entity_window));
    }

    #[test]
    fn initial_focus_not_overridden() {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputDispatchPlugin));

        app.world_mut().spawn((Window::default(), PrimaryWindow));

        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn(AutoFocus);
        });

        app.update();

        let autofocus_entity = app
            .world_mut()
            .query_filtered::<Entity, With<AutoFocus>>()
            .single(app.world())
            .unwrap();

        assert_eq!(
            app.world().resource::<InputFocus>().0,
            Some(autofocus_entity)
        );
    }

    #[test]
    fn test_keyboard_events() {
        fn get_gathered(app: &App, entity: Entity) -> &str {
            app.world()
                .entity(entity)
                .get::<GatherKeyboardEvents>()
                .unwrap()
                .0
                .as_str()
        }

        let mut app = App::new();

        app.add_plugins((InputPlugin, InputDispatchPlugin))
            .add_observer(gather_keyboard_events);

        app.world_mut().spawn((Window::default(), PrimaryWindow));

        // Run the world for a single frame to set up the initial focus
        app.update();

        let entity_a = app
            .world_mut()
            .spawn((GatherKeyboardEvents::default(), AutoFocus))
            .id();

        let child_of_b = app
            .world_mut()
            .spawn((GatherKeyboardEvents::default(),))
            .id();

        let entity_b = app
            .world_mut()
            .spawn((GatherKeyboardEvents::default(),))
            .add_child(child_of_b)
            .id();

        assert!(app.world().is_focused(entity_a));
        assert!(!app.world().is_focused(entity_b));
        assert!(!app.world().is_focused(child_of_b));
        assert!(!app.world().is_focus_visible(entity_a));
        assert!(!app.world().is_focus_visible(entity_b));
        assert!(!app.world().is_focus_visible(child_of_b));

        // entity_a should receive this event
        app.world_mut().write_message(key_a_message());
        app.update();

        assert_eq!(get_gathered(&app, entity_a), "A");
        assert_eq!(get_gathered(&app, entity_b), "");
        assert_eq!(get_gathered(&app, child_of_b), "");

        app.world_mut().insert_resource(InputFocus(None));

        assert!(!app.world().is_focused(entity_a));
        assert!(!app.world().is_focus_visible(entity_a));

        // This event should be lost
        app.world_mut().write_message(key_a_message());
        app.update();

        assert_eq!(get_gathered(&app, entity_a), "A");
        assert_eq!(get_gathered(&app, entity_b), "");
        assert_eq!(get_gathered(&app, child_of_b), "");

        app.world_mut()
            .insert_resource(InputFocus::from_entity(entity_b));
        assert!(app.world().is_focused(entity_b));
        assert!(!app.world().is_focused(child_of_b));

        app.world_mut()
            .run_system_once(move |mut input_focus: ResMut<InputFocus>| {
                input_focus.set(child_of_b);
            })
            .unwrap();
        assert!(app.world().is_focus_within(entity_b));

        // These events should be received by entity_b and child_of_b
        app.world_mut()
            .write_message_batch(core::iter::repeat_n(key_a_message(), 4));
        app.update();

        assert_eq!(get_gathered(&app, entity_a), "A");
        assert_eq!(get_gathered(&app, entity_b), "AAAA");
        assert_eq!(get_gathered(&app, child_of_b), "AAAA");

        app.world_mut().resource_mut::<InputFocusVisible>().0 = true;

        app.world_mut()
            .run_system_once(move |helper: IsFocusedHelper| {
                assert!(!helper.is_focused(entity_a));
                assert!(!helper.is_focus_within(entity_a));
                assert!(!helper.is_focus_visible(entity_a));
                assert!(!helper.is_focus_within_visible(entity_a));

                assert!(!helper.is_focused(entity_b));
                assert!(helper.is_focus_within(entity_b));
                assert!(!helper.is_focus_visible(entity_b));
                assert!(helper.is_focus_within_visible(entity_b));

                assert!(helper.is_focused(child_of_b));
                assert!(helper.is_focus_within(child_of_b));
                assert!(helper.is_focus_visible(child_of_b));
                assert!(helper.is_focus_within_visible(child_of_b));
            })
            .unwrap();

        app.world_mut()
            .run_system_once(move |world: DeferredWorld| {
                assert!(!world.is_focused(entity_a));
                assert!(!world.is_focus_within(entity_a));
                assert!(!world.is_focus_visible(entity_a));
                assert!(!world.is_focus_within_visible(entity_a));

                assert!(!world.is_focused(entity_b));
                assert!(world.is_focus_within(entity_b));
                assert!(!world.is_focus_visible(entity_b));
                assert!(world.is_focus_within_visible(entity_b));

                assert!(world.is_focused(child_of_b));
                assert!(world.is_focus_within(child_of_b));
                assert!(world.is_focus_visible(child_of_b));
                assert!(world.is_focus_within_visible(child_of_b));
            })
            .unwrap();
    }

    #[test]
    fn dispatch_clears_focus_when_focused_entity_despawned() {
        let mut app = App::new();
        app.add_plugins((InputPlugin, InputDispatchPlugin));

        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();

        let entity = app.world_mut().spawn_empty().id();
        app.world_mut()
            .insert_resource(InputFocus::from_entity(entity));
        app.world_mut().entity_mut(entity).despawn();

        assert_eq!(app.world().resource::<InputFocus>().0, Some(entity));

        // Send input event - this should clear focus instead of panicking
        app.world_mut().write_message(key_a_message());
        app.update();

        assert_eq!(app.world().resource::<InputFocus>().0, None);
    }
}
