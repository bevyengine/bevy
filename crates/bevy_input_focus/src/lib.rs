#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Keyboard focus system for Bevy.
//!
//! This crate provides a system for managing input focus in Bevy applications, including:
//! * [`InputFocus`], a resource for tracking which entity has input focus.
//! * Methods for getting and setting input focus via [`InputFocus`] and [`IsFocusedHelper`].
//! * A generic [`FocusedInput`] event for input events which bubble up from the focused entity.
//!
//! This crate does *not* provide any integration with UI widgets: this is the responsibility of the widget crate,
//! which should depend on [`bevy_input_focus`](crate).

pub mod tab_navigation;

// This module is too small / specific to be exported by the crate,
// but it's nice to have it separate for code organization.
mod autofocus;
pub use autofocus::*;

use bevy_app::{App, Plugin, PreUpdate, Startup};
use bevy_ecs::{prelude::*, query::QueryData, system::SystemParam, traversal::Traversal};
use bevy_hierarchy::{HierarchyQueryExt, Parent};
use bevy_input::{gamepad::GamepadButtonChangedEvent, keyboard::KeyboardInput, mouse::MouseWheel};
use bevy_window::{PrimaryWindow, Window};
use core::fmt::Debug;

/// Resource representing which entity has input focus, if any. Keyboard events will be
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
#[derive(Clone, Debug, Resource)]
pub struct InputFocusVisible(pub bool);

/// A bubble-able user input event that starts at the currently focused entity.
///
/// This event is normally dispatched to the current input focus entity, if any.
/// If no entity has input focus, then the event is dispatched to the main window.
///
/// To set up your own bubbling input event, add the [`dispatch_focused_input::<MyEvent>`](dispatch_focused_input) system to your app,
/// in the [`InputFocusSet::Dispatch`] system set during [`PreUpdate`].
#[derive(Clone, Debug, Component)]
pub struct FocusedInput<E: Event + Clone> {
    /// The underlying input event.
    pub input: E,
    /// The primary window entity.
    window: Entity,
}

impl<E: Event + Clone> Event for FocusedInput<E> {
    type Traversal = WindowTraversal;

    const AUTO_PROPAGATE: bool = true;
}

#[derive(QueryData)]
/// These are for accessing components defined on the targeted entity
pub struct WindowTraversal {
    parent: Option<&'static Parent>,
    window: Option<&'static Window>,
}

impl<E: Event + Clone> Traversal<FocusedInput<E>> for WindowTraversal {
    fn traverse(item: Self::Item<'_>, event: &FocusedInput<E>) -> Option<Entity> {
        let WindowTraversalItem { parent, window } = item;

        // Send event to parent, if it has one.
        if let Some(parent) = parent {
            return Some(parent.get());
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
        app.add_systems(Startup, set_initial_focus)
            .insert_resource(InputFocus(None))
            .insert_resource(InputFocusVisible(false))
            .add_systems(
                PreUpdate,
                (
                    dispatch_focused_input::<KeyboardInput>,
                    dispatch_focused_input::<GamepadButtonChangedEvent>,
                    dispatch_focused_input::<MouseWheel>,
                )
                    .in_set(InputFocusSet::Dispatch),
            );
    }
}

/// System sets for [`bevy_input_focus`](crate).
///
/// These systems run in the [`PreUpdate`] schedule.
#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
pub enum InputFocusSet {
    /// System which dispatches bubbled input events to the focused entity, or to the primary window.
    Dispatch,
}

/// Sets the initial focus to the primary window, if any.
pub fn set_initial_focus(
    mut input_focus: ResMut<InputFocus>,
    window: Single<Entity, With<PrimaryWindow>>,
) {
    input_focus.0 = Some(*window);
}

/// System which dispatches bubbled input events to the focused entity, or to the primary window
/// if no entity has focus.
pub fn dispatch_focused_input<E: Event + Clone>(
    mut key_events: EventReader<E>,
    focus: Res<InputFocus>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if let Ok(window) = windows.get_single() {
        // If an element has keyboard focus, then dispatch the input event to that element.
        if let Some(focused_entity) = focus.0 {
            for ev in key_events.read() {
                commands.trigger_targets(
                    FocusedInput {
                        input: ev.clone(),
                        window,
                    },
                    focused_entity,
                );
            }
        } else {
            // If no element has input focus, then dispatch the input event to the primary window.
            // There should be only one primary window.
            for ev in key_events.read() {
                commands.trigger_targets(
                    FocusedInput {
                        input: ev.clone(),
                        window,
                    },
                    window,
                );
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
    parent_query: Query<'w, 's, &'static Parent>,
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
            if let Some(parent) = self.entity(e).get::<Parent>().map(Parent::get) {
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

    use bevy_ecs::{
        component::ComponentId, observer::Trigger, system::RunSystemOnce, world::DeferredWorld,
    };
    use bevy_hierarchy::BuildChildren;
    use bevy_input::{
        keyboard::{Key, KeyCode},
        ButtonState, InputPlugin,
    };
    use bevy_window::WindowResolution;
    use smol_str::SmolStr;

    #[derive(Component)]
    #[component(on_add = set_focus_on_add)]
    struct SetFocusOnAdd;

    fn set_focus_on_add(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let mut input_focus = world.resource_mut::<InputFocus>();
        input_focus.set(entity);
    }

    #[derive(Component, Default)]
    struct GatherKeyboardEvents(String);

    fn gather_keyboard_events(
        trigger: Trigger<FocusedInput<KeyboardInput>>,
        mut query: Query<&mut GatherKeyboardEvents>,
    ) {
        if let Ok(mut gather) = query.get_mut(trigger.target()) {
            if let Key::Character(c) = &trigger.input.logical_key {
                gather.0.push_str(c.as_str());
            }
        }
    }

    const KEY_A_EVENT: KeyboardInput = KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character(SmolStr::new_static("A")),
        state: ButtonState::Pressed,
        text: Some(SmolStr::new_static("A")),
        repeat: false,
        window: Entity::PLACEHOLDER,
    };

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

        let window = Window {
            resolution: WindowResolution::new(800., 600.),
            ..Default::default()
        };
        app.world_mut().spawn((window, PrimaryWindow));

        // Run the world for a single frame to set up the initial focus
        app.update();

        let entity_a = app
            .world_mut()
            .spawn((GatherKeyboardEvents::default(), SetFocusOnAdd))
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
        app.world_mut().send_event(KEY_A_EVENT);
        app.update();

        assert_eq!(get_gathered(&app, entity_a), "A");
        assert_eq!(get_gathered(&app, entity_b), "");
        assert_eq!(get_gathered(&app, child_of_b), "");

        app.world_mut().insert_resource(InputFocus(None));

        assert!(!app.world().is_focused(entity_a));
        assert!(!app.world().is_focus_visible(entity_a));

        // This event should be lost
        app.world_mut().send_event(KEY_A_EVENT);
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
        app.world_mut().send_event_batch([KEY_A_EVENT; 4]);
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
}
