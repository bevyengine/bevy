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
//! + Hovering and movement: [`Over`], [`Enter`], [`Move`], [`Leave`], and [`Out`].
//! + Clicking and pressing: [`Press`], [`Release`], and [`Click`].
//! + Dragging and dropping: [`DragStart`], [`Drag`], [`DragEnd`], [`DragEnter`], [`DragOver`], [`DragDrop`], [`DragLeave`].
//!
//! When received by an observer, these events will always be wrapped by the [`Pointer`] type, which contains
//! general metadata about the pointer event.

use core::{fmt::Debug, time::Duration};
use std::collections::HashSet;

use bevy_camera::NormalizedRenderTarget;
use bevy_derive::{Deref, DerefMut};
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
    hover::{get_hovered_entities, is_directly_hovered, HoverMap, PreviousHoverMap},
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
    /// Whether to propagate the event via `PointerTraversal`
    /// For [`Enter`] and [`Leave`] events, this is set to false.
    pub(crate) propagate: bool,
}

/// A traversal query (i.e. it implements [`Traversal`]) intended for use with [`Pointer`] events.
///
/// Unless shortcircuited out by the [`Pointer`] event itself, this will always traverse to the
/// parent if the entity being visited has one. Otherwise, it propagates to the pointer's
/// window and stops there.
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
        if !pointer.propagate {
            return None;
        }

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
    /// Construct a new `Pointer<E>` event that propagates
    pub fn new(id: PointerId, location: Location, event: E, entity: Entity) -> Self {
        Self::new_inner(id, location, event, entity, true)
    }

    /// Construct a new `Pointer<E>` event that does not propagate
    pub fn new_without_propagate(
        id: PointerId,
        location: Location,
        event: E,
        entity: Entity,
    ) -> Self {
        Self::new_inner(id, location, event, entity, false)
    }

    fn new_inner(
        id: PointerId,
        location: Location,
        event: E,
        entity: Entity,
        propagate: bool,
    ) -> Self {
        Self {
            pointer_id: id,
            pointer_location: location,
            event,
            entity,
            propagate,
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

/// Fires when a pointer crosses into the bounds of a [target entity](EntityEvent::event_target).
/// Unlike [`Enter`], this event bubbles up to all of the
/// [target entity's](EntityEvent::event_target) ancestors (traversed via the [`ChildOf`] relationship)
/// without restriction. Refer to [`pointer_events`] for more information on how these events are triggered.
/// Refer to [`PointerTraversal`] for how [`Pointer`] events are propagated.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Over {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer crosses into the bounds of a [target entity](EntityEvent::event_target).
/// Unlike [`Over`], this event bubbles up through a subset of the
/// [target entity's](EntityEvent::event_target) ancestors
/// (traversed via the [`ChildOf`] relationship).
///
/// ### Event Propagation
/// An ancestor of a [target entity](EntityEvent::event_target) will receive an [`Enter`] event
/// when the ancestor does not have a direct relation to any entity hovered by the
/// pointer in the previous frame. For example, for a given pointer:
///
/// If the previously hovered entity C has the following entity ancestry: A -> B -> C
///
/// And the currently hovered entity E has the following entity ancestry: A -> D -> E
///
/// [`Enter`] events would be sent for both E and its direct ancestor D.
/// An [`Enter`] event would not be sent for A because it is a shared ancestor of both C and E.
///
/// Note: An [`Enter`] event may be fired for an ancestor even if the pointer does not enter
/// within the ancestor's bounds. More concretely, if a child's bounds extend beyond the parent's
/// and the pointer enters the child's bounds without crossing into the parent's,
/// two [`Enter`] events are still emitted for both the child and the parent.
/// This matches the triggering behavior of `mouseenter` events on the web.
/// To find out whether a pointer is within the target entity's bounds
/// immediately upon entering, check the value of [`is_in_bounds`](Enter::is_in_bounds).
///
/// Refer to [`pointer_events`] for more information on how these events are triggered.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Enter {
    /// Information about the picking intersection.
    pub hit: HitData,
    /// Whether this pointer directly entered into the target entity's bounds at the
    /// time of the event.
    /// This may be false if this entity's child's bounds extended beyond the entity and
    /// the pointer entered within the child's bounds only.
    pub is_in_bounds: bool,
}

/// Fires when a pointer crosses out of the bounds of a [target entity](EntityEvent::event_target).
/// Unlike [`Leave`], this event bubbles up to all of the
/// [target entity's](EntityEvent::event_target) ancestors (traversed via the [`ChildOf`] relationship)
/// without restriction. Refer to [`pointer_events`] for more information on how these events are triggered.
/// Refer to [`PointerTraversal`] for how [`Pointer`] events are propagated.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Out {
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer crosses out of the bounds of a [target entity](EntityEvent::event_target).
/// Unlike [`Out`], this event bubbles up through a subset of the
/// [target entity's](EntityEvent::event_target) ancestors
/// (traversed via the [`ChildOf`] relationship).
///
/// ### Event Propagation
/// An ancestor of a [target entity](EntityEvent::event_target) will receive a [`Leave`] event
/// when the ancestor does not have a direct relation to any entity hovered by the
/// pointer in the current frame. For example, for a given pointer:
///
/// If the previously hovered entity C has the following entity ancestry: A -> B -> C
///
/// And the currently hovered entity E has the following entity ancestry: A -> D -> E
///
/// [`Leave`] events would be sent for both C and its direct ancestor B.
/// A [`Leave`] event would not be sent for A because it is a shared ancestor of both C and E.
///
/// Note: A [`Leave`] event may be fired for an ancestor even if the pointer does not leave
/// the ancestor's bounds. More concretely, if a child's bounds extend beyond the parent's
/// and the pointer leaves from within those extended bounds,
/// two [`Leave`] events are still emitted for both the child and the parent.
/// This matches the triggering behavior of `mouseleave` events on the web.
/// To find out whether the pointer was within the target entity's bounds
/// right before leaving, check the value of [`was_in_bounds`](Leave::was_in_bounds).
///
/// Refer to [`pointer_events`] for more information on how these events are triggered.
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct Leave {
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
    /// Whether this pointer directly exited out of the target entity's bounds
    /// at the time of the event.
    /// This may be false if this entity's child's bounds extended beyond the entity and
    /// the pointer exited out of the child's bounds only.
    pub was_in_bounds: bool,
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

/// Fires when a pointer dragging the `dragged` entity enters the [target entity](EntityEvent::event_target)
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
    /// Stores the hit data for each entity currently being dragged over by the pointer.
    pub dragging_over: HashMap<Entity, HitData>,
}

impl PointerButtonState {
    /// Clears all press and drag data tracked for this button on its pointer.
    pub fn clear(&mut self) {
        self.pressing.clear();
        self.dragging.clear();
        self.dragging_over.clear();
    }
}

/// A cache map containing the ancestry of hovered entities
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct HoveredEntityAncestors(HashMap<Entity, HashSet<Entity>>);

impl HoveredEntityAncestors {
    /// Clears self and rebuilds a map of every hovered entity to its ancestors.
    ///
    /// This map is used to calculate which entities should receive [`Enter`] or [`Leave`] events.
    pub fn rebuild(
        &mut self,
        hover_map: &HoverMap,
        pointer_state: &PointerState,
        ancestors_query: &Query<&ChildOf>,
    ) {
        self.clear();
        for hovered_entity in hover_map
            .iter()
            .flat_map(|(_, hashmap)| hashmap.iter().map(|data| *data.0))
        {
            // If the ancestors were already added into the map, do not re-fetch
            if self.contains_key(&hovered_entity) {
                continue;
            }
            // If the ancestors were previously fetched, just re-use the entry.
            if let Some(previous_entry) =
                pointer_state.hovered_entity_ancestors.get(&hovered_entity)
            {
                self.insert(hovered_entity, previous_entry.clone());
            } else {
                let mut ancestors = HashSet::new();
                for member in ancestors_query.iter_ancestors(hovered_entity) {
                    ancestors.insert(member);
                }
                self.insert(hovered_entity, ancestors);
            }
        }
    }

    /// Returns a new combined `HashSet` of ancestors for the provided `hover_entities`
    pub fn get_ancestors_union(&self, hover_entities: &HashSet<Entity>) -> HashSet<Entity> {
        hover_entities
            .iter()
            .flat_map(|entity| self.get(entity))
            .flat_map(|set| set.iter().copied())
            .collect::<HashSet<Entity>>()
    }

    /// Returns the ancestors for the provided `hover_entity`, if it has been created
    pub fn get_ancestors(&self, hover_entity: &Entity) -> Option<&HashSet<Entity>> {
        self.get(hover_entity)
    }
}

/// State for all pointers.
#[derive(Debug, Clone, Default, Resource)]
pub struct PointerState {
    /// Pressing and dragging state, organized by pointer and button.
    pub pointer_buttons: HashMap<(PointerId, PointerButton), PointerButtonState>,
    /// A cache map providing the set of an entity's ancestors for a given hovered entity.
    pub hovered_entity_ancestors: HoveredEntityAncestors,
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

    /// Retrieves the ancestors for a given hovered entity
    pub fn get_ancestors(&self, hovered_entity: &Entity) -> Option<&HashSet<Entity>> {
        self.hovered_entity_ancestors.get_ancestors(hovered_entity)
    }

    /// Retrieves the union of ancestors for the given hovered entities
    pub fn get_ancestors_union(&self, hovered_entities: &HashSet<Entity>) -> HashSet<Entity> {
        self.hovered_entity_ancestors
            .get_ancestors_union(hovered_entities)
    }

    /// Clears all the data associated with all of the buttons on a pointer. Does not free the underlying memory.
    pub fn clear(&mut self, pointer_id: PointerId) {
        for button in PointerButton::iter() {
            if let Some(state) = self.pointer_buttons.get_mut(&(pointer_id, button)) {
                state.clear();
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
    leave_events: MessageWriter<'w, Pointer<Leave>>,
    enter_events: MessageWriter<'w, Pointer<Enter>>,
    released_events: MessageWriter<'w, Pointer<Release>>,
}

/// Dispatches interaction events to the target entities.
///
/// Within a single frame, events are dispatched in the following order:
/// + [`Out`] → [`Leave`] → [`DragLeave`].
/// + [`DragEnter`] → [`Enter`] → [`Over`].
/// + Any number of any of the following:
///   + For each movement: [`DragStart`] → [`Drag`] → [`DragOver`] → [`Move`].
///   + For each button press: [`Press`] or [`Click`] → [`Release`] → [`DragDrop`] → [`DragEnd`] → [`DragLeave`].
///   + For each pointer cancellation: [`Cancel`].
///
/// Additionally, across multiple frames, the following are also strictly
/// ordered by the interaction state machine:
/// + When a pointer moves over the target:
///   [`Over`], [`Enter`], [`Move`], [`Leave`], [`Out`].
/// + When a pointer presses buttons on the target:
///   [`Press`], [`Click`], [`Release`].
/// + When a pointer drags the target:
///   [`DragStart`], [`Drag`], [`DragEnd`].
/// + When a pointer drags something over the target:
///   [`DragEnter`], [`DragOver`], [`DragDrop`], [`DragLeave`].
/// + When a pointer is canceled:
///   No other events will follow the [`Cancel`] event for that pointer.
///
/// Four events -- [`Over`], [`Enter`], [`Leave`] and [`Out`] -- are driven only by the [`HoverMap`].
/// The rest rely on additional data from the [`PointerInput`] event stream. To
/// receive these events for a custom pointer, you must add [`PointerInput`]
/// events.
///
/// When the pointer goes from hovering entity A to entity B, entity A will
/// receive [`Out`] and [`Enter`] and then entity B will receive [`Leave`] and [`Over`].
/// No entity will ever receive both an [`Over`] and an [`Out`] or
/// an [`Enter`] and a [`Leave`] event during the same frame.
///
/// When we account for event bubbling, the two pairs of events,
/// [`Out`] [`Over`] and [`Enter`] [`Leave`], behave differently. When the hovering focus shifts
/// between children, parent entities may receive redundant [`Out`] → [`Over`] pairs. In
/// the case of [`Enter`] → [`Leave`], shared parent entities will not receive [`Enter`]
/// or [`Leave`].
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
    ancestors_query: Query<&ChildOf>,
    pointer_map: Res<PointerMap>,
    hover_map: Res<HoverMap>,
    previous_hover_map: Res<PreviousHoverMap>,
    mut pointer_state: ResMut<PointerState>,
    mut hovered_entity_ancestors: Local<HoveredEntityAncestors>,
    mut sent_leave: Local<HashSet<(PointerId, Entity)>>,
    mut sent_enter: Local<HashSet<(PointerId, Entity)>>,
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
    hovered_entity_ancestors.rebuild(&hover_map, &pointer_state, &ancestors_query);
    sent_leave.clear();
    sent_enter.clear();

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

            // Potentially send `Leave` events to the entity and all of its ancestors
            let mut entities_to_send_leave =
                pointer_state.get_ancestors(&hovered_entity).map_or_else(
                    || {
                        ancestors_query
                            .iter_ancestors(hovered_entity)
                            .collect::<HashSet<Entity>>()
                    },
                    Clone::clone,
                );
            entities_to_send_leave.insert(hovered_entity);
            // Ensure we do not double send to any other entities that have already been sent to during this loop
            entities_to_send_leave.retain(|entity| !sent_leave.contains(&(pointer_id, *entity)));
            if !entities_to_send_leave.is_empty() {
                // Fetch the currently hovered entities and their ancestors
                let new_hovered_entities = get_hovered_entities(&hover_map, &pointer_id);
                let new_hovered_ancestors =
                    hovered_entity_ancestors.get_ancestors_union(&new_hovered_entities);
                let union = new_hovered_entities
                    .union(&new_hovered_ancestors)
                    .copied()
                    .collect::<HashSet<Entity>>();
                // Keep entities and ancestors that are not going to continue to be hovered over
                entities_to_send_leave.retain(|entity| !union.contains(entity));
                // Send `Leave` events for those entities.
                // Note that `Leave` events send without propagation; we manually calculated
                // which ancestors should receive one.
                for leave_event in entities_to_send_leave.iter().map(|entity| {
                    Pointer::new_without_propagate(
                        pointer_id,
                        location.clone(),
                        Leave {
                            hit: hit.clone(),
                            was_in_bounds: is_directly_hovered(
                                &previous_hover_map.0,
                                &pointer_id,
                                entity,
                            ),
                        },
                        *entity,
                    )
                }) {
                    let entity = leave_event.entity;
                    commands.trigger(leave_event.clone());
                    message_writers.leave_events.write(leave_event);
                    sent_leave.insert((pointer_id, entity));
                }
            }

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

    // Iterate all currently hovered entities for each pointer
    for (pointer_id, hovered_entity, hit) in hover_map
        .iter()
        .flat_map(|(id, hashmap)| hashmap.iter().map(|data| (*id, *data.0, data.1.clone())))
    {
        // Continue if the pointer does not have a valid location.
        let Some(location) = pointer_location(pointer_id) else {
            debug!(
                "Unable to get location for pointer {:?} during pointer over",
                pointer_id
            );
            continue;
        };

        // For each button update its `dragging_over` state and possibly emit DragEnter events.
        for button in PointerButton::iter() {
            let state = pointer_state.get_mut(pointer_id, button);

            // Only update the `dragging_over` state if there is at least one entity being dragged.
            // Only emit DragEnter events for this `hovered_entity`, if it had no previous `dragging_over` state.
            if !state.dragging.is_empty()
                && state
                    .dragging_over
                    .insert(hovered_entity, hit.clone())
                    .is_none()
            {
                for drag_target in state.dragging.keys() {
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
        }

        // If the `hovered_entity` was not hovered by the same pointer the previous frame...
        if !previous_hover_map
            .get(&pointer_id)
            .iter()
            .any(|e| e.contains_key(&hovered_entity))
        {
            // Potentially send `Enter` events to the entity and all of its ancestors
            let mut entities_to_send_enter = hovered_entity_ancestors
                .get_ancestors(&hovered_entity)
                .map_or_else(
                    || {
                        ancestors_query
                            .iter_ancestors(hovered_entity)
                            .collect::<HashSet<Entity>>()
                    },
                    Clone::clone,
                );
            entities_to_send_enter.insert(hovered_entity);
            // Ensure we do not double send to any other entities that have already been sent to during this loop
            entities_to_send_enter
                .retain(|entity: &Entity| !sent_enter.contains(&(pointer_id, *entity)));
            if !entities_to_send_enter.is_empty() {
                // Fetch the previously hovered entities and their ancestors
                let prev_hovered_entities = get_hovered_entities(&previous_hover_map, &pointer_id);
                let prev_hovered_ancestors =
                    pointer_state.get_ancestors_union(&prev_hovered_entities);
                let union = prev_hovered_entities
                    .union(&prev_hovered_ancestors)
                    .copied()
                    .collect::<HashSet<Entity>>();
                // Keep entities and ancestors that were not hovered over previously
                entities_to_send_enter.retain(|entity| !union.contains(entity));
                // Send `Enter` events for those entities.
                // Note that `Enter` events send without propagation; we manually calculated
                // which ancestors should receive one.
                for enter_event in entities_to_send_enter.iter().map(|entity| {
                    Pointer::new_without_propagate(
                        pointer_id,
                        location.clone(),
                        Enter {
                            hit: hit.clone(),
                            is_in_bounds: is_directly_hovered(&hover_map.0, &pointer_id, entity),
                        },
                        *entity,
                    )
                }) {
                    let entity = enter_event.entity;
                    commands.trigger(enter_event.clone());
                    message_writers.enter_events.write(enter_event);
                    sent_enter.insert((pointer_id, entity));
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

    // Update pointer_state with the current hovered entity ancestors
    // We swap with the Local SystemParam's map, which will be rebuilt
    // on the next invocation of `pointer_events`
    core::mem::swap(
        &mut hovered_entity_ancestors.0,
        &mut pointer_state.hovered_entity_ancestors,
    );

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

                // Emit Click and Release events on all the previously hovered entities.
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
                state.clear();
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

                        // Insert dragging over state and emit DragEnter for hovered entities.
                        for (hovered_entity, hit) in hover_map
                            .get(&pointer_id)
                            .iter()
                            .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.to_owned())))
                            .filter(|(hovered_entity, _)| *hovered_entity != *press_target)
                        {
                            // Inserting the `dragging_over` state here ensures the `DragEnter` event won't be dispatched twice.
                            state.dragging_over.insert(hovered_entity, hit.clone());
                            let drag_enter_event = Pointer::new(
                                pointer_id,
                                location.clone(),
                                DragEnter {
                                    button,
                                    dragged: *press_target,
                                    hit: hit.clone(),
                                },
                                hovered_entity,
                            );
                            commands.trigger(drag_enter_event.clone());
                            message_writers.drag_enter_events.write(drag_enter_event);
                        }
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

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_camera::{Camera, ManualTextureViewHandle};

    use crate::pointer::update_pointer_map;

    use super::*;

    const POINTER_ID: PointerId = PointerId::Mouse;
    const STUB_LOCATION: Location = Location {
        target: NormalizedRenderTarget::TextureView(ManualTextureViewHandle(5)),
        position: Vec2::new(3., 4.),
    };

    fn initialize_app_for_test(app: &mut App) {
        // Init all the resources and messages necessary to run `pointer_events`
        app.init_resource::<HoverMap>()
            .init_resource::<PreviousHoverMap>()
            .init_resource::<PointerState>()
            .add_message::<PointerInput>()
            .add_message::<Pointer<Cancel>>()
            .add_message::<Pointer<Click>>()
            .add_message::<Pointer<Press>>()
            .add_message::<Pointer<DragDrop>>()
            .add_message::<Pointer<DragEnd>>()
            .add_message::<Pointer<DragEnter>>()
            .add_message::<Pointer<Drag>>()
            .add_message::<Pointer<DragLeave>>()
            .add_message::<Pointer<DragOver>>()
            .add_message::<Pointer<DragStart>>()
            .add_message::<Pointer<Scroll>>()
            .add_message::<Pointer<Move>>()
            .add_message::<Pointer<Out>>()
            .add_message::<Pointer<Over>>()
            .add_message::<Pointer<Leave>>()
            .add_message::<Pointer<Enter>>()
            .add_message::<Pointer<Release>>();

        // Initialize the pointer map resource manually with a stub location for the mouse
        app.world_mut()
            .spawn((POINTER_ID, PointerLocation::new(STUB_LOCATION)));
        app.world_mut().insert_resource(PointerMap::default());
        assert!(app
            .world_mut()
            .run_system_cached(update_pointer_map)
            .is_ok());
    }

    fn update_hover_map_with_hovered_entities(app: &mut App, camera: Entity, entities: &[Entity]) {
        let mut hover_map = HoverMap::default();
        let mut entity_map = HashMap::default();
        for entity in entities {
            entity_map.insert(
                *entity,
                HitData {
                    depth: 0.0,
                    camera,
                    position: None,
                    normal: None,
                },
            );
        }
        hover_map.insert(PointerId::Mouse, entity_map);

        let previous_hover_map = app.world().resource::<HoverMap>().0.clone();
        app.world_mut()
            .insert_resource(PreviousHoverMap(previous_hover_map));
        app.world_mut().insert_resource(hover_map);
    }

    #[test]
    fn enter_leave_events() {
        // the bool distinguishes between different *_in_bounds bool vals
        #[derive(Resource, Default)]
        struct EnterEventCounts(HashMap<(Entity, bool), usize>);

        #[derive(Resource, Default)]
        struct LeaveEventCounts(HashMap<(Entity, bool), usize>);

        fn observe_enter(event: On<Pointer<Enter>>, mut counts: ResMut<EnterEventCounts>) {
            *counts
                .0
                .entry((event.entity, event.event().is_in_bounds))
                .or_insert(0_usize) += 1;
        }

        fn observe_leave(event: On<Pointer<Leave>>, mut counts: ResMut<LeaveEventCounts>) {
            *counts
                .0
                .entry((event.entity, event.event().was_in_bounds))
                .or_insert(0_usize) += 1;
        }

        fn assert_msg_event_counts(app: &App, enter_count: usize, leave_count: usize) {
            let enter_messages = app.world().resource::<Messages<Pointer<Enter>>>();
            let leave_messages = app.world().resource::<Messages<Pointer<Leave>>>();
            assert_eq!(enter_messages.len(), enter_count);
            assert_eq!(leave_messages.len(), leave_count);
        }

        fn assert_observer_event_counts(
            app: &App,
            entity: Entity,
            enter_in_bounds_counts: usize,
            enter_out_of_bounds_counts: usize,
            leave_in_bounds_counts: usize,
            leave_out_of_bounds_counts: usize,
        ) {
            assert_eq!(
                *app.world()
                    .resource::<EnterEventCounts>()
                    .0
                    .get(&(entity, true))
                    .unwrap_or(&0),
                enter_in_bounds_counts
            );
            assert_eq!(
                *app.world()
                    .resource::<EnterEventCounts>()
                    .0
                    .get(&(entity, false))
                    .unwrap_or(&0),
                enter_out_of_bounds_counts
            );
            assert_eq!(
                *app.world()
                    .resource::<LeaveEventCounts>()
                    .0
                    .get(&(entity, true))
                    .unwrap_or(&0),
                leave_in_bounds_counts
            );
            assert_eq!(
                *app.world()
                    .resource::<LeaveEventCounts>()
                    .0
                    .get(&(entity, false))
                    .unwrap_or(&0),
                leave_out_of_bounds_counts
            );
        }

        let mut app = App::new();
        initialize_app_for_test(&mut app);
        app.init_resource::<EnterEventCounts>()
            .init_resource::<LeaveEventCounts>();
        let enter_messages = app.world().resource::<Messages<Pointer<Enter>>>();
        let leave_messages = app.world().resource::<Messages<Pointer<Leave>>>();
        assert_eq!(enter_messages.len(), 0);
        assert_eq!(leave_messages.len(), 0);
        // Setup test entities
        let camera = app.world_mut().spawn(Camera::default()).id();
        let child_one = app
            .world_mut()
            .spawn_empty()
            .observe(observe_enter)
            .observe(observe_leave)
            .id();
        let child_two = app
            .world_mut()
            .spawn_empty()
            .observe(observe_enter)
            .observe(observe_leave)
            .id();
        let parent = app
            .world_mut()
            .spawn_empty()
            .add_children(&[child_one, child_two])
            .observe(observe_enter)
            .observe(observe_leave)
            .id();

        // FIRST: child_one is hovered over
        update_hover_map_with_hovered_entities(&mut app, camera, &[child_one]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // child_one received an in_bounds `Enter` event
        // The parent received an indirect `Enter` event because its child was hovered into
        assert_msg_event_counts(&app, 2, 0);
        assert_observer_event_counts(&app, parent, 0, 1, 0, 0);
        assert_observer_event_counts(&app, child_one, 1, 0, 0, 0);
        assert_observer_event_counts(&app, child_two, 0, 0, 0, 0);
        app.world_mut().increment_change_tick();
        // ---

        // SECOND: child_one is hovered out of, child_two and parent are directly hovered over
        update_hover_map_with_hovered_entities(&mut app, camera, &[child_two, parent]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // child_one received an in_bounds `Leave` event.
        // child_two received an in_bounds `Enter` event.
        // The parent did not receive any events because it is a shared ancestor
        assert_msg_event_counts(&app, 3, 1);
        assert_observer_event_counts(&app, parent, 0, 1, 0, 0);
        assert_observer_event_counts(&app, child_one, 1, 0, 1, 0);
        assert_observer_event_counts(&app, child_two, 1, 0, 0, 0);
        app.world_mut().increment_change_tick();
        // ---

        // THIRD: child_two is hovered out of, parent is still hovered
        update_hover_map_with_hovered_entities(&mut app, camera, &[parent]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // child_two received an in_bounds `Leave` event.
        assert_msg_event_counts(&app, 3, 2);
        assert_observer_event_counts(&app, parent, 0, 1, 0, 0);
        assert_observer_event_counts(&app, child_one, 1, 0, 1, 0);
        assert_observer_event_counts(&app, child_two, 1, 0, 1, 0);
        app.world_mut().increment_change_tick();
        // ---

        // FOURTH: child_two is hovered back into, parent is no longer directly hovered
        update_hover_map_with_hovered_entities(&mut app, camera, &[child_two]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // child_two received an in_bounds `Enter` event
        // The parent did not receive an `Leave` event because its child is still hovered
        assert_msg_event_counts(&app, 4, 2);
        assert_observer_event_counts(&app, parent, 0, 1, 0, 0);
        assert_observer_event_counts(&app, child_one, 1, 0, 1, 0);
        assert_observer_event_counts(&app, child_two, 2, 0, 1, 0);
        app.world_mut().increment_change_tick();
        // ---

        // FIFTH: child_two is hovered out of
        update_hover_map_with_hovered_entities(&mut app, camera, &[]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // child_two received one in_bounds `Leave` event
        // The parent received one indirect `Leave` event because the pointer is no longer hovering
        // any of its children
        assert_msg_event_counts(&app, 4, 4);
        assert_observer_event_counts(&app, parent, 0, 1, 0, 1);
        assert_observer_event_counts(&app, child_one, 1, 0, 1, 0);
        assert_observer_event_counts(&app, child_two, 2, 0, 2, 0);
        app.world_mut().increment_change_tick();
        // ---

        // FINAL: parent and child_one are directly hovered into
        update_hover_map_with_hovered_entities(&mut app, camera, &[parent, child_one]);

        assert!(app.world_mut().run_system_cached(pointer_events).is_ok());

        // The parent received one in_bounds `Enter` event
        // child_one received one in_bounds `Enter` event
        assert_msg_event_counts(&app, 6, 4);
        assert_observer_event_counts(&app, parent, 1, 1, 0, 1);
        assert_observer_event_counts(&app, child_one, 2, 0, 1, 0);
        assert_observer_event_counts(&app, child_two, 2, 0, 2, 0);
        app.world_mut().increment_change_tick();
        // ---
    }
}
