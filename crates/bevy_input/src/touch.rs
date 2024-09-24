//! The touch input functionality.

use bevy_ecs::entity::Entity;
use bevy_ecs::event::{Event, EventReader};
use bevy_ecs::system::{ResMut, Resource};
use bevy_math::Vec2;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::HashMap;

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A touch input event.
///
/// ## Logic
///
/// Every time the user touches the screen, a new [`TouchPhase::Started`] event with an unique
/// identifier for the finger is generated. When the finger is lifted, the [`TouchPhase::Ended`]
/// event is generated with the same finger id.
///
/// After a [`TouchPhase::Started`] event has been emitted, there may be zero or more [`TouchPhase::Moved`]
/// events when the finger is moved or the touch pressure changes.
///
/// The finger id may be reused by the system after an [`TouchPhase::Ended`] event. The user
/// should assume that a new [`TouchPhase::Started`] event received with the same id has nothing
/// to do with the old finger and is a new finger.
///
/// A [`TouchPhase::Canceled`] event is emitted when the system has canceled tracking this
/// touch, such as when the window loses focus, or on iOS if the user moves the
/// device against their face.
///
/// ## Note
///
/// This event is the translated version of the `WindowEvent::Touch` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
#[derive(Event, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct TouchInput {
    /// The phase of the touch input.
    pub phase: TouchPhase,
    /// The position of the finger on the touchscreen.
    pub position: Vec2,
    /// The window entity registering the touch.
    pub window: Entity,
    /// Describes how hard the screen was pressed.
    ///
    /// May be [`None`] if the platform does not support pressure sensitivity.
    /// This feature is only available on **iOS** 9.0+ and **Windows** 8+.
    pub force: Option<ForceTouch>,
    /// The unique identifier of the finger.
    pub id: u64,
}

/// A force description of a [`Touch`] input.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum ForceTouch {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really hard, or not hard at all, depending on the device.
    Normalized(f64),
}

/// A phase of a [`TouchInput`].
///
/// ## Usage
///
/// It is used to describe the phase of the touch input that is currently active.
/// This includes a phase that indicates that a touch input has started or ended,
/// or that a finger has moved. There is also a canceled phase that indicates that
/// the system canceled the tracking of the finger.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum TouchPhase {
    /// A finger started to touch the touchscreen.
    Started,
    /// A finger moved over the touchscreen.
    Moved,
    /// A finger stopped touching the touchscreen.
    Ended,
    /// The system canceled the tracking of the finger.
    ///
    /// This occurs when the window loses focus, or on iOS if the user moves the
    /// device against their face.
    Canceled,
}

/// A touch input.
///
/// ## Usage
///
/// It is used to store the position and force of a touch input and also the `id` of the finger.
/// The data of the touch input comes from the [`TouchInput`] event and is being stored
/// inside of the [`Touches`] `bevy` resource.
#[derive(Debug, Clone, Copy)]
pub struct Touch {
    /// The id of the touch input.
    id: u64,
    /// The starting position of the touch input.
    start_position: Vec2,
    /// The starting force of the touch input.
    start_force: Option<ForceTouch>,
    /// The previous position of the touch input.
    previous_position: Vec2,
    /// The previous force of the touch input.
    previous_force: Option<ForceTouch>,
    /// The current position of the touch input.
    position: Vec2,
    /// The current force of the touch input.
    force: Option<ForceTouch>,
}

impl Touch {
    /// The delta of the current `position` and the `previous_position`.
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    /// The distance of the `start_position` and the current `position`.
    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }

    /// Returns the `id` of the touch.
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the `start_position` of the touch.
    #[inline]
    pub fn start_position(&self) -> Vec2 {
        self.start_position
    }

    /// Returns the `start_force` of the touch.
    #[inline]
    pub fn start_force(&self) -> Option<ForceTouch> {
        self.start_force
    }

    /// Returns the `previous_position` of the touch.
    #[inline]
    pub fn previous_position(&self) -> Vec2 {
        self.previous_position
    }

    /// Returns the `previous_force` of the touch.
    #[inline]
    pub fn previous_force(&self) -> Option<ForceTouch> {
        self.previous_force
    }

    /// Returns the current `position` of the touch.
    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// Returns the current `force` of the touch.
    #[inline]
    pub fn force(&self) -> Option<ForceTouch> {
        self.force
    }
}

impl From<&TouchInput> for Touch {
    fn from(input: &TouchInput) -> Touch {
        Touch {
            id: input.id,
            start_position: input.position,
            start_force: input.force,
            previous_position: input.position,
            previous_force: input.force,
            position: input.position,
            force: input.force,
        }
    }
}

/// A collection of [`Touch`]es.
///
/// ## Usage
///
/// It is used to create a `bevy` resource that stores the data of the touches on a touchscreen
/// and can be accessed inside of a system.
///
/// ## Updating
///
/// The resource is updated inside of the [`touch_screen_input_system`].
#[derive(Debug, Clone, Default, Resource)]
pub struct Touches {
    /// A collection of every [`Touch`] that is currently being pressed.
    pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got pressed.
    just_pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got released.
    just_released: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got canceled.
    just_canceled: HashMap<u64, Touch>,
}

impl Touches {
    /// An iterator visiting every pressed [`Touch`] input in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.pressed.values()
    }

    /// Returns the [`Touch`] input corresponding to the `id` if it is being pressed.
    pub fn get_pressed(&self, id: u64) -> Option<&Touch> {
        self.pressed.get(&id)
    }

    /// Checks if any touch input was just pressed.
    pub fn any_just_pressed(&self) -> bool {
        !self.just_pressed.is_empty()
    }

    /// Register a release for a given touch input.
    pub fn release(&mut self, id: u64) {
        if let Some(touch) = self.pressed.remove(&id) {
            self.just_released.insert(id, touch);
        }
    }

    /// Registers a release for all currently pressed touch inputs.
    pub fn release_all(&mut self) {
        self.just_released.extend(self.pressed.drain());
    }

    /// Returns `true` if the input corresponding to the `id` has just been pressed.
    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains_key(&id)
    }

    /// Clears the `just_pressed` state of the touch input and returns `true` if the touch input has just been pressed.
    ///
    /// Future calls to [`Touches::just_pressed`] for the given touch input will return false until a new press event occurs.
    pub fn clear_just_pressed(&mut self, id: u64) -> bool {
        self.just_pressed.remove(&id).is_some()
    }

    /// An iterator visiting every just pressed [`Touch`] input in arbitrary order.
    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> {
        self.just_pressed.values()
    }

    /// Returns the [`Touch`] input corresponding to the `id` if it has just been released.
    pub fn get_released(&self, id: u64) -> Option<&Touch> {
        self.just_released.get(&id)
    }

    /// Checks if any touch input was just released.
    pub fn any_just_released(&self) -> bool {
        !self.just_released.is_empty()
    }

    /// Returns `true` if the input corresponding to the `id` has just been released.
    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains_key(&id)
    }

    /// Clears the `just_released` state of the touch input and returns `true` if the touch input has just been released.
    ///
    /// Future calls to [`Touches::just_released`] for the given touch input will return false until a new release event occurs.
    pub fn clear_just_released(&mut self, id: u64) -> bool {
        self.just_released.remove(&id).is_some()
    }

    /// An iterator visiting every just released [`Touch`] input in arbitrary order.
    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> {
        self.just_released.values()
    }

    /// Checks if any touch input was just canceled.
    pub fn any_just_canceled(&self) -> bool {
        !self.just_canceled.is_empty()
    }

    /// Returns `true` if the input corresponding to the `id` has just been canceled.
    pub fn just_canceled(&self, id: u64) -> bool {
        self.just_canceled.contains_key(&id)
    }

    /// Clears the `just_canceled` state of the touch input and returns `true` if the touch input has just been canceled.
    ///
    /// Future calls to [`Touches::just_canceled`] for the given touch input will return false until a new cancel event occurs.
    pub fn clear_just_canceled(&mut self, id: u64) -> bool {
        self.just_canceled.remove(&id).is_some()
    }

    /// An iterator visiting every just canceled [`Touch`] input in arbitrary order.
    pub fn iter_just_canceled(&self) -> impl Iterator<Item = &Touch> {
        self.just_canceled.values()
    }

    /// Retrieves the position of the first currently pressed touch, if any
    pub fn first_pressed_position(&self) -> Option<Vec2> {
        // Looking for the position in `pressed`. If nothing is found, also look into `just_pressed`
        // A touch can be in `just_pressed` but not in `pressed` if it ended in the same frame it started
        self.pressed
            .values()
            .next()
            .or_else(|| self.just_pressed.values().next())
            .map(|t| t.position)
    }

    /// Clears `just_pressed`, `just_released`, and `just_canceled` data for every touch input.
    ///
    /// See also [`Touches::reset_all`] for a full reset.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_canceled.clear();
    }

    /// Clears `pressed`, `just_pressed`, `just_released`, and `just_canceled` data for every touch input.
    ///
    /// See also [`Touches::clear`] for clearing only touches that have just been pressed, released or canceled.
    pub fn reset_all(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_canceled.clear();
    }

    /// Processes a [`TouchInput`] event by updating the `pressed`, `just_pressed`,
    /// `just_released`, and `just_canceled` collections.
    fn process_touch_event(&mut self, event: &TouchInput) {
        match event.phase {
            TouchPhase::Started => {
                self.pressed.insert(event.id, event.into());
                self.just_pressed.insert(event.id, event.into());
            }
            TouchPhase::Moved => {
                if let Some(mut new_touch) = self.pressed.get(&event.id).cloned() {
                    // NOTE: This does not update the previous_force / previous_position field;
                    // they should be updated once per frame, not once per event
                    // See https://github.com/bevyengine/bevy/issues/12442
                    new_touch.position = event.position;
                    new_touch.force = event.force;
                    self.pressed.insert(event.id, new_touch);
                }
            }
            TouchPhase::Ended => {
                // if touch `just_released`, add related event to it
                // the event position info is inside `pressed`, so use it unless not found
                if let Some((_, v)) = self.pressed.remove_entry(&event.id) {
                    self.just_released.insert(event.id, v);
                } else {
                    self.just_released.insert(event.id, event.into());
                }
            }
            TouchPhase::Canceled => {
                // if touch `just_canceled`, add related event to it
                // the event position info is inside `pressed`, so use it unless not found
                if let Some((_, v)) = self.pressed.remove_entry(&event.id) {
                    self.just_canceled.insert(event.id, v);
                } else {
                    self.just_canceled.insert(event.id, event.into());
                }
            }
        };
    }
}

/// Updates the [`Touches`] resource with the latest [`TouchInput`] events.
///
/// This is not clearing the `pressed` collection, because it could incorrectly mark a touch input
/// as not pressed even though it is pressed. This could happen if the touch input is not moving
/// for a single frame and would therefore be marked as not pressed, because this function is
/// called on every single frame no matter if there was an event or not.
///
/// ## Differences
///
/// The main difference between the [`TouchInput`] event and the [`Touches`] resource is that
/// the latter has convenient functions like [`Touches::just_pressed`] and [`Touches::just_released`].
pub fn touch_screen_input_system(
    mut touch_state: ResMut<Touches>,
    mut touch_input_events: EventReader<TouchInput>,
) {
    if !touch_state.just_pressed.is_empty() {
        touch_state.just_pressed.clear();
    }
    if !touch_state.just_released.is_empty() {
        touch_state.just_released.clear();
    }
    if !touch_state.just_canceled.is_empty() {
        touch_state.just_canceled.clear();
    }

    if !touch_input_events.is_empty() {
        for touch in touch_state.pressed.values_mut() {
            touch.previous_position = touch.position;
            touch.previous_force = touch.force;
        }

        for event in touch_input_events.read() {
            touch_state.process_touch_event(event);
        }
    }
}

#[cfg(test)]
mod test {
    use super::Touches;

    #[test]
    fn touch_update() {
        use crate::{touch::Touch, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = Touch {
            id: 4,
            start_position: Vec2::ZERO,
            start_force: None,
            previous_position: Vec2::ZERO,
            previous_force: None,
            position: Vec2::ZERO,
            force: None,
        };

        // Add a touch to `just_pressed`, 'just_released', and 'just canceled'

        touches.just_pressed.insert(4, touch_event);
        touches.just_released.insert(4, touch_event);
        touches.just_canceled.insert(4, touch_event);

        clear_all(&mut touches);

        // Verify that all the `just_x` maps are cleared
        assert!(touches.just_pressed.is_empty());
        assert!(touches.just_released.is_empty());
        assert!(touches.just_canceled.is_empty());
    }

    #[test]
    fn touch_process() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        // Test adding a `TouchPhase::Started`

        let touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        clear_all(&mut touches);
        touches.process_touch_event(&touch_event);

        assert!(touches.pressed.get(&touch_event.id).is_some());
        assert!(touches.just_pressed.get(&touch_event.id).is_some());

        // Test adding a `TouchPhase::Moved`

        let moved_touch_event = TouchInput {
            phase: TouchPhase::Moved,
            position: Vec2::splat(5.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: touch_event.id,
        };

        clear_all(&mut touches);
        touches.process_touch_event(&moved_touch_event);

        assert_eq!(
            touches
                .pressed
                .get(&moved_touch_event.id)
                .expect("Missing from pressed after move.")
                .previous_position,
            touch_event.position
        );

        // Test cancelling an event

        let cancel_touch_event = TouchInput {
            phase: TouchPhase::Canceled,
            position: Vec2::ONE,
            window: Entity::PLACEHOLDER,
            force: None,
            id: touch_event.id,
        };

        clear_all(&mut touches);
        touches.process_touch_event(&cancel_touch_event);

        assert!(touches.just_canceled.get(&touch_event.id).is_some());
        assert!(touches.pressed.get(&touch_event.id).is_none());

        // Test ending an event

        let end_touch_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: touch_event.id,
        };

        clear_all(&mut touches);
        touches.process_touch_event(&touch_event);
        touches.process_touch_event(&moved_touch_event);
        touches.process_touch_event(&end_touch_event);

        assert!(touches.just_released.get(&touch_event.id).is_some());
        assert!(touches.pressed.get(&touch_event.id).is_none());
        let touch = touches.just_released.get(&touch_event.id).unwrap();
        // Make sure the position is updated from TouchPhase::Moved and TouchPhase::Ended
        assert_ne!(touch.previous_position, touch.position);
    }

    // See https://github.com/bevyengine/bevy/issues/12442
    #[test]
    fn touch_process_multi_event() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let started_touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        let moved_touch_event1 = TouchInput {
            phase: TouchPhase::Moved,
            position: Vec2::splat(5.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: started_touch_event.id,
        };

        let moved_touch_event2 = TouchInput {
            phase: TouchPhase::Moved,
            position: Vec2::splat(6.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: started_touch_event.id,
        };

        // tick 1: touch is started during frame
        for touch in touches.pressed.values_mut() {
            // update ONCE, at start of frame
            touch.previous_position = touch.position;
        }
        touches.process_touch_event(&started_touch_event);
        touches.process_touch_event(&moved_touch_event1);
        touches.process_touch_event(&moved_touch_event2);

        {
            let touch = touches.get_pressed(started_touch_event.id).unwrap();
            assert_eq!(touch.previous_position, started_touch_event.position);
            assert_eq!(touch.position, moved_touch_event2.position);
        }

        // tick 2: touch was started before frame
        for touch in touches.pressed.values_mut() {
            touch.previous_position = touch.position;
        }
        touches.process_touch_event(&moved_touch_event1);
        touches.process_touch_event(&moved_touch_event2);
        touches.process_touch_event(&moved_touch_event1);

        {
            let touch = touches.get_pressed(started_touch_event.id).unwrap();
            assert_eq!(touch.previous_position, moved_touch_event2.position);
            assert_eq!(touch.position, moved_touch_event1.position);
        }
    }

    #[test]
    fn touch_pressed() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.get_pressed(touch_event.id).is_some());
        assert!(touches.just_pressed(touch_event.id));
        assert_eq!(touches.iter().count(), 1);

        touches.clear_just_pressed(touch_event.id);
        assert!(!touches.just_pressed(touch_event.id));
    }

    #[test]
    fn touch_released() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.get_released(touch_event.id).is_some());
        assert!(touches.just_released(touch_event.id));
        assert_eq!(touches.iter_just_released().count(), 1);

        touches.clear_just_released(touch_event.id);
        assert!(!touches.just_released(touch_event.id));
    }

    #[test]
    fn touch_canceled() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Canceled,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.just_canceled(touch_event.id));
        assert_eq!(touches.iter_just_canceled().count(), 1);

        touches.clear_just_canceled(touch_event.id);
        assert!(!touches.just_canceled(touch_event.id));
    }

    #[test]
    fn release_touch() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.get_pressed(touch_event.id).is_some());

        touches.release(touch_event.id);
        assert!(touches.get_pressed(touch_event.id).is_none());
        assert!(touches.just_released(touch_event.id));
    }

    #[test]
    fn release_all_touches() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_pressed_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        let touch_moved_event = TouchInput {
            phase: TouchPhase::Moved,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        touches.process_touch_event(&touch_pressed_event);
        touches.process_touch_event(&touch_moved_event);

        assert!(touches.get_pressed(touch_pressed_event.id).is_some());
        assert!(touches.get_pressed(touch_moved_event.id).is_some());

        touches.release_all();

        assert!(touches.get_pressed(touch_pressed_event.id).is_none());
        assert!(touches.just_released(touch_pressed_event.id));
        assert!(touches.get_pressed(touch_moved_event.id).is_none());
        assert!(touches.just_released(touch_moved_event.id));
    }

    #[test]
    fn clear_touches() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_press_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        let touch_canceled_event = TouchInput {
            phase: TouchPhase::Canceled,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 5,
        };

        let touch_released_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 6,
        };

        // Register the touches and test that it was registered correctly
        touches.process_touch_event(&touch_press_event);
        touches.process_touch_event(&touch_canceled_event);
        touches.process_touch_event(&touch_released_event);

        assert!(touches.get_pressed(touch_press_event.id).is_some());
        assert!(touches.just_pressed(touch_press_event.id));
        assert!(touches.just_canceled(touch_canceled_event.id));
        assert!(touches.just_released(touch_released_event.id));

        touches.clear();

        assert!(touches.get_pressed(touch_press_event.id).is_some());
        assert!(!touches.just_pressed(touch_press_event.id));
        assert!(!touches.just_canceled(touch_canceled_event.id));
        assert!(!touches.just_released(touch_released_event.id));
    }

    #[test]
    fn reset_all_touches() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_ecs::entity::Entity;
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_press_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 4,
        };

        let touch_canceled_event = TouchInput {
            phase: TouchPhase::Canceled,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 5,
        };

        let touch_released_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::splat(4.0),
            window: Entity::PLACEHOLDER,
            force: None,
            id: 6,
        };

        // Register the touches and test that it was registered correctly
        touches.process_touch_event(&touch_press_event);
        touches.process_touch_event(&touch_canceled_event);
        touches.process_touch_event(&touch_released_event);

        assert!(touches.get_pressed(touch_press_event.id).is_some());
        assert!(touches.just_pressed(touch_press_event.id));
        assert!(touches.just_canceled(touch_canceled_event.id));
        assert!(touches.just_released(touch_released_event.id));

        touches.reset_all();

        assert!(touches.get_pressed(touch_press_event.id).is_none());
        assert!(!touches.just_pressed(touch_press_event.id));
        assert!(!touches.just_canceled(touch_canceled_event.id));
        assert!(!touches.just_released(touch_released_event.id));
    }

    fn clear_all(touch_state: &mut Touches) {
        touch_state.just_pressed.clear();
        touch_state.just_released.clear();
        touch_state.just_canceled.clear();
    }
}
