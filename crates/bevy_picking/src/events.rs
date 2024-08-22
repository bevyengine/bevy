//! Processes data from input and backends, producing interaction events.

use std::fmt::Debug;

use bevy_ecs::prelude::*;
use bevy_hierarchy::Parent;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use bevy_utils::{tracing::debug, Duration, HashMap, Instant};

use crate::{
    backend::{prelude::PointerLocation, HitData},
    focus::{HoverMap, PreviousHoverMap},
    pointer::{
        Location, PointerAction, PointerButton, PointerId, PointerInput, PointerMap, PressDirection,
    },
};

/// Stores the common data needed for all `PointerEvent`s.
#[derive(Clone, PartialEq, Debug, Reflect, Component)]
pub struct Pointer<E: Debug + Clone + Reflect> {
    /// The pointer that triggered this event
    pub pointer_id: PointerId,
    /// The location of the pointer during this event
    pub pointer_location: Location,
    /// Additional event-specific data. [`Drop`] for example, has an additional field to describe
    /// the `Entity` that is being dropped on the target.
    pub event: E,
}

impl<E> Event for Pointer<E>
where
    E: Debug + Clone + Reflect,
{
    type Traversal = Parent;

    const AUTO_PROPAGATE: bool = true;
}

impl<E: Debug + Clone + Reflect> std::fmt::Display for Pointer<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:?}, {:.1?}, {:.1?}",
            self.pointer_id, self.pointer_location.position, self.event
        ))
    }
}

impl<E: Debug + Clone + Reflect> std::ops::Deref for Pointer<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl<E: Debug + Clone + Reflect> Pointer<E> {
    /// Construct a new `PointerEvent`.
    pub fn new(id: PointerId, location: Location, event: E) -> Self {
        Self {
            pointer_id: id,
            pointer_location: location,
            event,
        }
    }
}

/// Fires when a the pointer crosses into the bounds of the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Over {
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a the pointer crosses out of the bounds of the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Out {
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer button is pressed over the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Down {
    /// Pointer button pressed to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer button is released over the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Up {
    /// Pointer button lifted to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer sends a pointer down event followed by a pointer up event, with the same
/// `target` entity for both events.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Click {
    /// Pointer button pressed and lifted to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
    /// Duration between the pointer pressed and lifted for this click
    pub duration: Duration,
}

/// Fires while a pointer is moving over the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Move {
    /// Information about the picking intersection.
    pub hit: HitData,
    /// The change in position since the last move event.
    pub delta: Vec2,
}

/// Fires when the `target` entity receives a pointer down event followed by a pointer move event.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct DragStart {
    /// Pointer button pressed and moved to trigger this event.
    pub button: PointerButton,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires while the `target` entity is being dragged.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Drag {
    /// Pointer button pressed and moved to trigger this event.
    pub button: PointerButton,
    /// The total distance vector of a drag, measured from drag start to the current position.
    pub distance: Vec2,
    /// The change in position since the last drag event.
    pub delta: Vec2,
}

/// Fires when a pointer is dragging the `target` entity and a pointer up event is received.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct DragEnd {
    /// Pointer button pressed, moved, and lifted to trigger this event.
    pub button: PointerButton,
    /// The vector of drag movement measured from start to final pointer position.
    pub distance: Vec2,
}

/// Fires when a pointer dragging the `dragged` entity enters the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct DragEnter {
    /// Pointer button pressed to enter drag.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer entered the `target` entity.
    pub dragged: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires while the `dragged` entity is being dragged over the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct DragOver {
    /// Pointer button pressed while dragging over.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer was over the `target` entity.
    pub dragged: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer dragging the `dragged` entity leaves the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct DragLeave {
    /// Pointer button pressed while leaving drag.
    pub button: PointerButton,
    /// The entity that was being dragged when the pointer left the `target` entity.
    pub dragged: Entity,
    /// Information about the latest prior picking intersection.
    pub hit: HitData,
}

/// Fires when a pointer drops the `dropped` entity onto the `target` entity.
#[derive(Clone, PartialEq, Debug, Reflect)]
pub struct Drop {
    /// Pointer button lifted to drop.
    pub button: PointerButton,
    /// The entity that was dropped onto the `target` entity.
    pub dropped: Entity,
    /// Information about the picking intersection.
    pub hit: HitData,
}

/// An entry in the [`DragMap`].
#[derive(Debug, Clone)]
pub struct DragEntry {
    /// The position of the pointer at drag start.
    pub start_pos: Vec2,
    /// The latest position of the pointer during this drag, used to compute deltas.
    pub latest_pos: Vec2,
}

/// Dispatches interaction events to entities.
///
/// Events will be dispatched in the following order:
/// + The sequence Over, DragEnter
/// + Any number of any of the following:
///   + For each movement: The sequence Move, DragStart, Drag, DragOver
///   + For each button press: Either Down, or the sequence Up, Click, DragEnd, Drop, DragLeave
/// + Finally the sequence DragLeave, Out
///
/// Additionally, the following are guaranteed to be received in the order by each listener:
/// + When a pointer moves over the target: Over, Move, Out,
/// + When a pointer presses buttons on the target: Down, Up, Click
/// + When a pointer drags the target: DragStart, Drag, DragEnd
/// + When a pointer drags something over the target: DragEnter, DragOver, Drop, DragLeave
pub fn pointer_events(
    // Input
    mut input_events: EventReader<PointerInput>,
    // ECS State
    pointers: Query<&PointerLocation>,
    pointer_map: Res<PointerMap>,
    hover_map: Res<HoverMap>,
    previous_hover_map: Res<PreviousHoverMap>,
    // Local state
    mut drag_map: Local<HashMap<(PointerId, PointerButton), HashMap<Entity, DragEntry>>>,
    mut drag_over_map: Local<HashMap<(PointerId, PointerButton), HashMap<Entity, HitData>>>,
    mut down_map: Local<
        HashMap<(PointerId, PointerButton), HashMap<Entity, (Pointer<Down>, Instant)>>,
    >,
    // Output
    mut commands: Commands,
) {
    // Setup utilities
    let now = Instant::now();
    let pointer_location = |pointer_id: PointerId| {
        pointer_map
            .get_entity(pointer_id)
            .and_then(|entity| pointers.get(entity).ok())
            .and_then(|pointer| pointer.location.clone())
    };

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
            // Send Over events
            commands.trigger_targets(
                Pointer::new(pointer_id, location.clone(), Over { hit: hit.clone() }),
                hovered_entity,
            );
            // Possibly send DragEnter events
            for button in PointerButton::iter() {
                for drag_target in drag_map
                    .get(&(pointer_id, button))
                    .iter()
                    .flat_map(|drag_list| drag_list.keys())
                    .filter(
                        |&&drag_target| hovered_entity != drag_target, /* can't drag over itself */
                    )
                {
                    let drag_entry = drag_over_map.entry((pointer_id, button)).or_default();
                    drag_entry.insert(hovered_entity, hit.clone());
                    commands.trigger_targets(
                        Pointer::new(
                            pointer_id,
                            location.clone(),
                            DragEnter {
                                button,
                                dragged: *drag_target,
                                hit: hit.clone(),
                            },
                        ),
                        hovered_entity,
                    );
                }
            }
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
            // Entered Window
            PointerAction::EnteredWindow => todo!(),
            // Left Window
            PointerAction::LeftWindow => todo!(),
            // Pressed Button
            PointerAction::Pressed { direction, button } => {
                // Send Up, Down, and Click button events to hovered entity
                for (hovered_entity, hit) in previous_hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.clone())))
                {
                    match direction {
                        PressDirection::Down => {
                            // Send the Down event first
                            let event =
                                Pointer::new(pointer_id, location.clone(), Down { button, hit });
                            commands.trigger_targets(event.clone(), hovered_entity);
                            // Also update the down map
                            let down_button_entity_map =
                                down_map.entry((pointer_id, button)).or_default();
                            down_button_entity_map.insert(hovered_entity, (event, now));
                        }
                        PressDirection::Up => {
                            // Send the Up event first
                            commands.trigger_targets(
                                Pointer::new(
                                    pointer_id,
                                    location.clone(),
                                    Up {
                                        button,
                                        hit: hit.clone(),
                                    },
                                ),
                                hovered_entity,
                            );
                            // If this pointer previously pressed the hovered entity, also emit a Click event
                            if let Some((_down, down_instant)) = down_map
                                .get(&(pointer_id, button))
                                .and_then(|down| down.get(&hovered_entity))
                            {
                                commands.trigger_targets(
                                    Pointer::new(
                                        pointer_id,
                                        location.clone(),
                                        Click {
                                            button,
                                            hit: hit.clone(),
                                            duration: now - *down_instant,
                                        },
                                    ),
                                    hovered_entity,
                                )
                            }
                        }
                    };
                }

                // Additionally, for all button releases clear out the state and possibly emit DragEnd, Drop, DragLeave and
                if direction == PressDirection::Up {
                    down_map.insert((pointer_id, button), HashMap::new());
                    let Some(drag_list) = drag_map.insert((pointer_id, button), HashMap::new())
                    else {
                        continue;
                    };
                    for (drag_target, drag) in drag_list {
                        // Emit DragEnd
                        commands.trigger_targets(
                            Pointer::new(
                                pointer_id,
                                location.clone(),
                                DragEnd {
                                    button,
                                    distance: drag.latest_pos - drag.start_pos,
                                },
                            ),
                            drag_target,
                        );
                        // Emit Drop and DragLeave
                        let Some(drag_over_set) = drag_over_map.get_mut(&(pointer_id, button))
                        else {
                            continue;
                        };
                        for (dragged_over, hit) in drag_over_set.drain() {
                            commands.trigger_targets(
                                Pointer::new(
                                    pointer_id,
                                    location.clone(),
                                    Drop {
                                        button,
                                        dropped: drag_target,
                                        hit: hit.clone(),
                                    },
                                ),
                                dragged_over,
                            );
                            commands.trigger_targets(
                                Pointer::new(
                                    pointer_id,
                                    location.clone(),
                                    DragLeave {
                                        button,
                                        dragged: drag_target,
                                        hit: hit.clone(),
                                    },
                                ),
                                dragged_over,
                            );
                        }
                    }
                }
            }
            // Moved
            PointerAction::Moved { delta } => {
                for (hovered_entity, hit) in hover_map
                    .get(&pointer_id)
                    .iter()
                    .flat_map(|h| h.iter().map(|(entity, data)| (*entity, data.to_owned())))
                {
                    // Send move events to hovered entity
                    commands.trigger_targets(
                        Pointer::new(
                            pointer_id,
                            location.clone(),
                            Move {
                                hit: hit.clone(),
                                delta,
                            },
                        ),
                        hovered_entity,
                    );

                    // Send drag events to entities being pressed
                    for button in PointerButton::iter() {
                        let Some(down_list) = down_map.get(&(pointer_id, button)) else {
                            continue;
                        };
                        let drag_list = drag_map.entry((pointer_id, button)).or_default();

                        // Emit a DragStart to the hovered entity when a pointer moves with a button pressed down, unless
                        // the pointer is already registered as dragged with that button.
                        for (down, _instant) in down_list.values() {
                            if drag_list.contains_key(&hovered_entity) {
                                continue; // this entity is already logged as being dragged
                            }
                            drag_list.insert(
                                hovered_entity,
                                DragEntry {
                                    start_pos: down.pointer_location.position,
                                    latest_pos: down.pointer_location.position,
                                },
                            );
                            commands.trigger_targets(
                                Pointer::new(
                                    pointer_id,
                                    location.clone(),
                                    DragStart {
                                        button,
                                        hit: hit.clone(),
                                    },
                                ),
                                hovered_entity,
                            );
                        }

                        // Emit a Drag event to the dragged entity when it is dragged over another entity.
                        for (dragged_entity, drag) in drag_list.iter_mut() {
                            let drag_event = Drag {
                                button,
                                distance: location.position - drag.start_pos,
                                delta: location.position - drag.latest_pos,
                            };
                            drag.latest_pos = location.position;
                            let target = *dragged_entity;
                            let event = Pointer::new(pointer_id, location.clone(), drag_event);
                            commands.trigger_targets(event, target);
                        }

                        // Emit a DragOver to the hovered entity when dragging a different entity over it.
                        for drag_target in drag_map
                            .get(&(pointer_id, button))
                            .iter()
                            .flat_map(|drag_list| drag_list.keys())
                            .filter(
                                |&&drag_target| hovered_entity != drag_target, /* can't drag over itself */
                            )
                        {
                            commands.trigger_targets(
                                Pointer::new(pointer_id, location.clone(), DragOver { button, dragged: *drag_target, hit: hit.clone() }),
                                hovered_entity,
                            );
                        }
                    }
                }
            }
            // Canceled
            PointerAction::Canceled => todo!(),
        }
    }

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
            // Possibly send DragLeave events
            for button in PointerButton::iter() {
                let Some(dragged_over) = drag_over_map.get_mut(&(pointer_id, button)) else {
                    continue;
                };
                if dragged_over.remove(&hovered_entity).is_none() {
                    continue;
                }
                let Some(drag_list) = drag_map.get(&(pointer_id, button)) else {
                    continue;
                };
                for drag_target in drag_list.keys() {
                    commands.trigger_targets(
                        Pointer::new(
                            pointer_id,
                            location.clone(),
                            DragLeave {
                                button,
                                dragged: *drag_target,
                                hit: hit.clone(),
                            },
                        ),
                        hovered_entity,
                    );
                }
            }
            // Send Out events
            commands.trigger_targets(
                Pointer::new(pointer_id, location.clone(), Out { hit: hit.clone() }),
                hovered_entity,
            );
        }
    }
}
