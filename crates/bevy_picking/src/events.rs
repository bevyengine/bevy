//! This module defines a stateful set of interaction events driven by the `PointerInput` stream
//! and the hover state of each Pointer.
//!
//! # Usage
//!
//! To receive events from this module, you must use an [`Observer`] or [`MessageReader`] with [`Pointer<E>`] events.
//! The simplest example, registering a callback when an entity is hovered over by a pointer, looks like this:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_picking::prelude::*;
//! # let mut world = World::default();
//! world.spawn_empty()
//!     .observe(|event: On<Pointer<Over>>| {
//!         println!("I am being hovered over");
//!     });
//! ```
//!
//! Observers give us three important properties:
//! 1. They allow for attaching event handlers to specific entities,
//! 2. they allow events to bubble up the entity hierarchy,
//! 3. and they allow events of different types to be called in a specific order.
//!
//! The order in which interaction events are received is extremely important, and you can read more
//! about it on the docs for the dispatcher system: [`pointer_events`]. This system runs in
//! [`PreUpdate`](bevy_app::PreUpdate) in [`PickingSystems::Hover`](crate::PickingSystems::Hover). All pointer-event
//! observers resolve during the sync point between [`pointer_events`] and
//! [`update_interactions`](crate::hover::update_interactions).
//!
//! # Events Types
//!
//! The events this module defines fall into a few broad categories:
//! + Hovering and movement: [`Over`], [`Move`], and [`Out`].
//! + Clicking and pressing: [`Press`], [`Release`], and [`Click`].
//! + Dragging and dropping: [`DragStart`], [`Drag`], [`DragEnd`], [`DragEnter`], [`DragOver`], [`DragDrop`], [`DragLeave`].
//!
//! When received by an observer, these events will always be wrapped by the [`Pointer`] type, which contains
//! general metadata about the pointer event.

use core::{fmt::Debug, time::Duration};

use bevy_camera::NormalizedRenderTarget;
use bevy_ecs::{prelude::*, query::QueryData, system::SystemParam, traversal::Traversal};
use bevy_input::mouse::MouseScrollUnit;
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
use bevy_platform::time::Instant;
use bevy_reflect::prelude::*;
use bevy_window::Window;
use tracing::debug;

use crate::{
    backend::{prelude::PointerLocation, HitData},
    hover::{HoverMap, PreviousHoverMap},
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput, PointerMap},
};

/// Stores the common data needed for all pointer events.
///
/// The documentation for the [`pointer_events`] explains the events this module exposes and
/// the order in which they fire.
#[derive(Message, EntityEvent, Clone, PartialEq, Debug, Reflect, Component)]
#[entity_event(propagate = PointerTraversal, auto_propagate)]
#[reflect(Component, Debug, Clone)]
pub struct Pointer<E: Debug + Clone + Reflect> {
    /// The entity this pointer event happened for.
    pub entity: Entity,
    /// The pointer that triggered this event
    pub pointer_id: PointerId,
    /// The location of the pointer during this event
    pub pointer_location: Location,
    /// Additional event-specific data. [`DragDrop`] for example, has an additional field to describe
    /// the `Entity` that is being dropped on the target.
    pub event: E,
}

/// A traversal query (i.e. it implements [`Traversal`]) intended for use with [`Pointer`] events.
///
/// This will always traverse to the parent, if the entity being visited has one. Otherwise, it
/// propagates to the pointer's window and stops there.
#[derive(QueryData)]
pub struct PointerTraversal {
    child_of: Option<&'static ChildOf>,
    window: Option<&'static Window>,
}

impl<E> Traversal<Pointer<E>> for PointerTraversal
where
    E: Debug + Clone + Reflect,
{
    fn traverse(item: Self::Item<'_, '_>, pointer: &Pointer<E>) -> Option<Entity> {
        let PointerTraversalItem { child_of, window } = item;

        // Send event to parent, if it has one.
        if let Some(child_of) = child_of {
            return Some(child_of.parent());
        };

        // Otherwise, send it to the window entity (unless this is a window entity).
        if window.is_none()
            && let NormalizedRenderTarget::Window(window_ref) = pointer.pointer_location.target
        {
            return Some(window_ref.entity());
        }

        None
    }
}

impl<E: Debug + Clone + Reflect> core::fmt::Display for Pointer<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{:?}, {:.1?}, {:.1?}",
            self.pointer_id, self.pointer_location.position, self.event
        ))
    }
}

impl<E: Debug + Clone + Reflect> core::ops::Deref for Pointer<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl<E: Debug + Clone + Reflect> Pointer<E> {
    /// Construct a new `Pointer<E>` event.
    pub fn new(id: PointerId, location: Location, event: E, entity: Entity) -> Self {
        Self {
            pointer_id: id,
            pointer_location: location,
            event,
            entity,
        }
    }
}

/// Fires when a pointer is canceled, and its current interaction state is dropped.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Cancel {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer crosses into the bounds of the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Over {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer crosses out of the bounds of the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Out {
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer button is pressed over the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Press {
    /// Pointer button pressed to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer button is released over the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Release {
    /// Pointer button lifted to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer sends a pointer pressed event followed by a pointer released event, with the same
/// [target entity](EntityEvent::event_target) for both events.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Click {
    /// Pointer button pressed and lifted to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
    /// Duration between the pointer pressed and lifted for this click
    pub duration: Duration,
}

/// Fires while a pointer is moving over the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Move {
    /// Information about the picking intersection.
    pub hit: HitData,
    /// The change in position since the last move event.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using methods on [`Camera`](bevy_camera::Camera) to convert from screen-space to
    /// world-space.
    pub delta: Vec2,
}

/// Fires when the [target entity](EntityEvent::event_target) receives a pointer pressed event followed by a pointer move event.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragStart {
    /// Pointer button pressed and moved to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires while the [target entity](EntityEvent::event_target) is being dragged.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Drag {
    /// Pointer button pressed and moved to trigger this event.
    pub button: PointerButton,
    /// The total distance vector of a drag, measured from drag start to the current position.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using methods on [`Camera`](bevy_camera::Camera) to convert from screen-space to
    /// world-space.
    pub distance: Vec2,
    /// The change in position since the last drag event.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using methods on [`Camera`](bevy_camera::Camera) to convert from screen-space to
    /// world-space.
    pub delta: Vec2,
}

/// Fires when a pointer is dragging the [target entity](EntityEvent::event_target) and a pointer released event is received.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragEnd {
    /// Pointer button pressed, moved, and released to trigger this event.
    pub button: PointerButton,
    /// The vector of drag movement measured from start to final pointer position.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using methods on [`Camera`](bevy_camera::Camera) to convert from screen-space to
    /// world-space.
    pub distance: Vec2,
}

/// Fires when a pointer dragging the `dragged` entity enters the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragEnter {
    /// Pointer button pressed to enter drag.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer entered the [target entity](EntityEvent::event_target).
    pub dragged: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires while the `dragged` entity is being dragged over the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragOver {
    /// Pointer button pressed while dragging over.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer was over the [target entity](EntityEvent::event_target).
    pub dragged: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer dragging the `dragged` entity leaves the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragLeave {
    /// Pointer button pressed while leaving drag.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer left the [target entity](EntityEvent::event_target).
    pub dragged: Entity,
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer drops the `dropped` entity onto the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragDrop {
    /// Pointer button released to drop.
    pub button: PointerButton,
    /// The entity that was dropped onto the [target entity](EntityEvent::event_target).
    pub dropped: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Dragging state.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct DragEntry {
    /// The position of the pointer at drag start.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using [`Camera::viewport_to_world`](bevy_camera::Camera::viewport_to_world) or
    /// [`Camera::viewport_to_world_2d`](bevy_camera::Camera::viewport_to_world_2d) to
    /// convert from screen-space to world-space.
    pub start_pos: Vec2,
    /// The latest position of the pointer during this drag, used to compute deltas.
    ///
    /// This is stored in screen pixels, not world coordinates. Screen pixels go from top-left to
    /// bottom-right, whereas (in 2D) world coordinates go from bottom-left to top-right. Consider
    /// using [`Camera::viewport_to_world`](bevy_camera::Camera::viewport_to_world) or
    /// [`Camera::viewport_to_world_2d`](bevy_camera::Camera::viewport_to_world_2d) to
    /// convert from screen-space to world-space.
    pub latest_pos: Vec2,
}

/// Fires while a pointer is scrolling over the [target entity](EntityEvent::event_target).
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Scroll {
    /// The mouse scroll unit.
    pub unit: MouseScrollUnit,
    /// The horizontal scroll value.
    pub x: f32,
    /// The vertical scroll value.
    pub y: f32,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// An entry in the cache that drives the `pointer_events` system, storing additional data
/// about pointer button presses.
#[derive(Debug, Clone, Default)]
pub struct PointerButtonState {
    /// Stores the press location and start time for each button currently being pressed by the pointer.
    pub pressing: HashMap<Entity, (Location, Instant, HitData)>,
    /// Stores the starting and current locations for each entity currently being dragged by the pointer.
    pub dragging: HashMap<Entity, DragEntry>,
    /// Stores  the hit data for each entity currently being dragged over by the pointer.
    pub dragging_over: HashMap<Entity, HitData>,
}

/// State for all pointers.
#[derive(Debug, Clone, Default, Resource)]
pub struct PointerState {
    /// Pressing and dragging state, organized by pointer and button.
    pub pointer_buttons: HashMap<(PointerId, PointerButton), PointerButtonState>,
}

impl PointerState {
    /// Retrieves the current state for a specific pointer and button, if it has been created.
    pub fn get(&self, pointer_id: PointerId, button: PointerButton) -> Option<&PointerButtonState> {
        self.pointer_buttons.get(&(pointer_id, button))
    }

    /// Provides write access to the state of a pointer and button, creating it if it does not yet exist.
    pub fn get_mut(
        &mut self,
        pointer_id: PointerId,
        button: PointerButton,
    ) -> &mut PointerButtonState {
        self.pointer_buttons
            .entry((pointer_id, button))
            .or_default()
    }

    /// Clears all the data associated with all of the buttons on a pointer. Does not free the underlying memory.
    pub fn clear(&mut self, pointer_id: PointerId) {
        for button in PointerButton::iter() {
            if let Some(state) = self.pointer_buttons.get_mut(&(pointer_id, button)) {
                state.pressing.clear();
                state.dragging.clear();
                state.dragging_over.clear();
            }
        }
    }
}

/// A helper system param for accessing the picking event writers.
#[derive(SystemParam)]
pub struct PickingMessageWriters<'w> {
    cancel_events: MessageWriter<'w, Pointer<Cancel>>,
    click_events: MessageWriter<'w, Pointer<Click>>,
    pressed_events: MessageWriter<'w, Pointer<Press>>,
    drag_drop_events: MessageWriter<'w, Pointer<DragDrop>>,
    drag_end_events: MessageWriter<'w, Pointer<DragEnd>>,
    drag_enter_events: MessageWriter<'w, Pointer<DragEnter>>,
    drag_events: MessageWriter<'w, Pointer<Drag>>,
    drag_leave_events: MessageWriter<'w, Pointer<DragLeave>>,
    drag_over_events: MessageWriter<'w, Pointer<DragOver>>,
    drag_start_events: MessageWriter<'w, Pointer<DragStart>>,
    scroll_events: MessageWriter<'w, Pointer<Scroll>>,
    move_events: MessageWriter<'w, Pointer<Move>>,
    out_events: MessageWriter<'w, Pointer<Out>>,
    over_events: MessageWriter<'w, Pointer<Over>>,
    released_events: MessageWriter<'w, Pointer<Release>>,
}

/// Dispatches interaction events to the target entities.
///
/// Within a single frame, events are dispatched in the following order:
/// + [`Out`] → [`DragLeave`].
/// + [`DragEnter`] → [`Over`].
/// + Any number of any of the following:
///   + For each movement: [`DragStart`] → [`Drag`] → [`DragOver`] → [`Move`].
///   + For each button press: [`Press`] or [`Click`] → [`Release`] → [`DragDrop`] → [`DragEnd`] → [`DragLeave`].
///   + For each pointer cancellation: [`Cancel`].
///
/// Additionally, across multiple frames, the following are also strictly
/// ordered by the interaction state machine:
/// + When a pointer moves over the target:
///   [`Over`], [`Move`], [`Out`].
/// + When a pointer presses buttons on the target:
///   [`Press`], [`Click`], [`Release`].
/// + When a pointer drags the target:
///   [`DragStart`], [`Drag`], [`DragEnd`].
/// + When a pointer drags something over the target:
///   [`DragEnter`], [`DragOver`], [`DragDrop`], [`DragLeave`].
/// + When a pointer is canceled:
///   No other events will follow the [`Cancel`] event for that pointer.
///
/// Two events -- [`Over`] and [`Out`] -- are driven only by the [`HoverMap`].
/// The rest rely on additional data from the [`PointerInput`] event stream. To
/// receive these events for a custom pointer, you must add [`PointerInput`]
/// events.
///
/// When the pointer goes from hovering entity A to entity B, entity A will
/// receive [`Out`] and then entity B will receive [`Over`]. No entity will ever
/// receive both an [`Over`] and and a [`Out`] event during the same frame.
///
/// When we account for event bubbling, this is no longer true. When the hovering focus shifts
/// between children, parent entities may receive redundant [`Out`] → [`Over`] pairs.
/// In the context of UI, this is especially problematic. Additional hierarchy-aware
/// events will be added in a future release.
///
/// Both [`Click`] and [`Release`] target the entity hovered in the *previous frame*,
/// rather than the current frame. This is because touch pointers hover nothing
/// on the frame they are released. The end effect is that these two events can
/// be received sequentially after an [`Out`] event (but always on the same frame
/// as the [`Out`] event).
///
/// Note: Though it is common for the [`PointerInput`] stream may contain
/// multiple pointer movements and presses each frame, the hover state is
/// determined only by the pointer's *final position*. Since the hover state
/// ultimately determines which entities receive events, this may mean that an
/// entity can receive events from before or after it was actually hovered.
pub fn pointer_events(
    // Input
    mut input_events: MessageReader<PointerInput>,
    // ECS State
    pointers: Query<&PointerLocation>,
    pointer_map: Res<PointerMap>,
    hover_map: Res<HoverMap>,
    previous_hover_map: Res<PreviousHoverMap>,
    mut pointer_state: ResMut<PointerState>,
    // Output
    mut commands: Commands,
    mut message_writers: PickingMessageWriters,
) {
    // Setup utilities
    let now = Instant::now();
    let pointer_location = |pointer_id: PointerId| {
        pointer_map
            .get_entity(pointer_id)
            .and_then(|entity| pointers.get(entity).ok())
            .and_then(|pointer| pointer.location.clone())
    };

    // If the entity was hovered by a specific pointer last frame...
    for (pointer_id, hovered_entity, hit) in previous_hover_map
        .iter()
        .flat_map(|(id, hashmap)| hashmap.iter().map(|data| (*id, *data.0, data.1.clone())))
    {
        // ...but is now not being hovered by that same pointer...
        if !hover_map
            .get(&pointer_id)
            .iter()
            .any(|e| e.contains_key(&hovered_entity))
        {
            let Some(location) = pointer_location(pointer_id) else {
                debug!(
                    "Unable to get location for pointer {:?} during pointer out",
                    pointer_id
                );
                continue;
            };

            // Always send Out events
            let out_event = Pointer::new(
                pointer_id,
                location.clone(),
                Out { hit: hit.clone() },
                hovered_entity,
            );
            commands.trigger(out_event.clone());
            message_writers.out_events.write(out_event);

            // Possibly send DragLeave events
            for button in PointerButton::iter() {
                let state = pointer_state.get_mut(pointer_id, button);
                state.dragging_over.remove(&hovered_entity);
                for drag_target in state.dragging.keys() {
                    let drag_leave_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        DragLeave {
                            button,
                            dragged: *drag_target,
                            hit: hit.clone(),
                        },
                        hovered_entity,
                    );
                    commands.trigger(drag_leave_event.clone());
                    message_writers.drag_leave_events.write(drag_leave_event);
                }
            }
        }
    }

    // If the entity is hovered...
    for (pointer_id, hovered_entity, hit) in hover_map
        .iter()
        .flat_map(|(id, hashmap)| hashmap.iter().map(|data| (*id, *data.0, data.1.clone())))
    {
        // ...but was not hovered last frame...
        if !previous_hover_map
            .get(&pointer_id)
            .iter()
            .any(|e| e.contains_key(&hovered_entity))
        {
            let Some(location) = pointer_location(pointer_id) else {
                debug!(
                    "Unable to get location for pointer {:?} during pointer over",
                    pointer_id
                );
                continue;
            };

            // Possibly send DragEnter events
            for button in PointerButton::iter() {
                let state = pointer_state.get_mut(pointer_id, button);

                for drag_target in state.dragging.keys() {
                    state.dragging_over.insert(hovered_entity, hit.clone());
                    let drag_enter_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        DragEnter {
                            button,
                            dragged: *drag_target,
                            hit: hit.clone(),
                        },
                        hovered_entity,
                    );
                    commands.trigger(drag_enter_event.clone());
                    message_writers.drag_enter_events.write(drag_enter_event);
                }
            }

            // Always send Over events
            let over_event = Pointer::new(
                pointer_id,
                location.clone(),
                Over { hit: hit.clone() },
                hovered_entity,
            );
            commands.trigger(over_event.clone());
            message_writers.over_events.write(over_event);
        }
    }

    // Dispatch input events...
    for PointerInput {
        pointer_id,
        location,
        action,
    } in input_events.read().cloned()
    {
        match action {
            PointerAction::Press(button) => {
                let state = pointer_state.get_mut(pointer_id, button);

                // If it's a press, emit a Pressed event and mark the hovered entities as pressed
                for (hovered_entity, hit) in hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.clone())))
                {
                    let pressed_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        Press {
                            button,
                            hit: hit.clone(),
                        },
                        hovered_entity,
                    );
                    commands.trigger(pressed_event.clone());
                    message_writers.pressed_events.write(pressed_event);
                    // Also insert the press into the state
                    state
                        .pressing
                        .insert(hovered_entity, (location.clone(), now, hit));
                }
            }
            PointerAction::Release(button) => {
                let state = pointer_state.get_mut(pointer_id, button);

                // Emit Click and Up events on all the previously hovered entities.
                for (hovered_entity, hit) in previous_hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.clone())))
                {
                    // If this pointer previously pressed the hovered entity, emit a Click event
                    if let Some((_, press_instant, _)) = state.pressing.get(&hovered_entity) {
                        let click_event = Pointer::new(
                            pointer_id,
                            location.clone(),
                            Click {
                                button,
                                hit: hit.clone(),
                                duration: now - *press_instant,
                            },
                            hovered_entity,
                        );
                        commands.trigger(click_event.clone());
                        message_writers.click_events.write(click_event);
                    }
                    // Always send the Release event
                    let released_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        Release {
                            button,
                            hit: hit.clone(),
                        },
                        hovered_entity,
                    );
                    commands.trigger(released_event.clone());
                    message_writers.released_events.write(released_event);
                }

                // Then emit the drop events.
                for (drag_target, drag) in state.dragging.drain() {
                    // Emit DragDrop
                    for (dragged_over, hit) in state.dragging_over.iter() {
                        let drag_drop_event = Pointer::new(
                            pointer_id,
                            location.clone(),
                            DragDrop {
                                button,
                                dropped: drag_target,
                                hit: hit.clone(),
                            },
                            *dragged_over,
                        );
                        commands.trigger(drag_drop_event.clone());
                        message_writers.drag_drop_events.write(drag_drop_event);
                    }
                    // Emit DragEnd
                    let drag_end_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        DragEnd {
                            button,
                            distance: drag.latest_pos - drag.start_pos,
                        },
                        drag_target,
                    );
                    commands.trigger(drag_end_event.clone());
                    message_writers.drag_end_events.write(drag_end_event);
                    // Emit DragLeave
                    for (dragged_over, hit) in state.dragging_over.iter() {
                        let drag_leave_event = Pointer::new(
                            pointer_id,
                            location.clone(),
                            DragLeave {
                                button,
                                dragged: drag_target,
                                hit: hit.clone(),
                            },
                            *dragged_over,
                        );
                        commands.trigger(drag_leave_event.clone());
                        message_writers.drag_leave_events.write(drag_leave_event);
                    }
                }

                // Finally, we can clear the state of everything relating to presses or drags.
                state.pressing.clear();
                state.dragging.clear();
                state.dragging_over.clear();
            }
            // Moved
            PointerAction::Move { delta } => {
                if delta == Vec2::ZERO {
                    continue; // If delta is zero, the following events will not be triggered.
                }
                // Triggers during movement even if not over an entity
                for button in PointerButton::iter() {
                    let state = pointer_state.get_mut(pointer_id, button);

                    // Emit DragEntry and DragStart the first time we move while pressing an entity
                    for (press_target, (location, _, hit)) in state.pressing.iter() {
                        if state.dragging.contains_key(press_target) {
                            continue; // This entity is already logged as being dragged
                        }
                        state.dragging.insert(
                            *press_target,
                            DragEntry {
                                start_pos: location.position,
                                latest_pos: location.position,
                            },
                        );
                        let drag_start_event = Pointer::new(
                            pointer_id,
                            location.clone(),
                            DragStart {
                                button,
                                hit: hit.clone(),
                            },
                            *press_target,
                        );
                        commands.trigger(drag_start_event.clone());
                        message_writers.drag_start_events.write(drag_start_event);
                    }

                    // Emit Drag events to the entities we are dragging
                    for (drag_target, drag) in state.dragging.iter_mut() {
                        let delta = location.position - drag.latest_pos;
                        if delta == Vec2::ZERO {
                            continue; // No need to emit a Drag event if there is no movement
                        }
                        let drag_event = Pointer::new(
                            pointer_id,
                            location.clone(),
                            Drag {
                                button,
                                distance: location.position - drag.start_pos,
                                delta,
                            },
                            *drag_target,
                        );
                        commands.trigger(drag_event.clone());
                        message_writers.drag_events.write(drag_event);

                        // Update drag position
                        drag.latest_pos = location.position;

                        // Emit corresponding DragOver to the hovered entities
                        for (hovered_entity, hit) in hover_map
                            .get(&pointer_id)
                            .iter()
                            .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.to_owned())))
                            .filter(|(hovered_entity, _)| *hovered_entity != *drag_target)
                        {
                            let drag_over_event = Pointer::new(
                                pointer_id,
                                location.clone(),
                                DragOver {
                                    button,
                                    dragged: *drag_target,
                                    hit: hit.clone(),
                                },
                                hovered_entity,
                            );
                            commands.trigger(drag_over_event.clone());
                            message_writers.drag_over_events.write(drag_over_event);
                        }
                    }
                }

                for (hovered_entity, hit) in hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.to_owned())))
                {
                    // Emit Move events to the entities we are hovering
                    let move_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        Move {
                            hit: hit.clone(),
                            delta,
                        },
                        hovered_entity,
                    );
                    commands.trigger(move_event.clone());
                    message_writers.move_events.write(move_event);
                }
            }
            PointerAction::Scroll { x, y, unit } => {
                for (hovered_entity, hit) in hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.clone())))
                {
                    // Emit Scroll events to the entities we are hovering
                    let scroll_event = Pointer::new(
                        pointer_id,
                        location.clone(),
                        Scroll {
                            unit,
                            x,
                            y,
                            hit: hit.clone(),
                        },
                        hovered_entity,
                    );
                    commands.trigger(scroll_event.clone());
                    message_writers.scroll_events.write(scroll_event);
                }
            }
            // Canceled
            PointerAction::Cancel => {
                // Emit a Cancel to the hovered entity.
                for (hovered_entity, hit) in hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.to_owned())))
                {
                    let cancel_event =
                        Pointer::new(pointer_id, location.clone(), Cancel { hit }, hovered_entity);
                    commands.trigger(cancel_event.clone());
                    message_writers.cancel_events.write(cancel_event);
                }
                // Clear the state for the canceled pointer
                pointer_state.clear(pointer_id);
            }
        }
    }
}
