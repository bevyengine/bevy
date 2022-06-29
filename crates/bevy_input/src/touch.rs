use bevy_ecs::event::EventReader;
use bevy_ecs::system::ResMut;
use bevy_math::Vec2;
use bevy_utils::HashMap;

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
/// A [`TouchPhase::Cancelled`] event is emitted when the system has canceled tracking this
/// touch, such as when the window loses focus, or on iOS if the user moves the
/// device against their face.
///
/// ## Note
///
/// This event is the translated version of the `WindowEvent::Touch` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchInput {
    /// The phase of the touch input.
    pub phase: TouchPhase,
    /// The position of the finger on the touchscreen.
    pub position: Vec2,
    /// Describes how hard the screen was pressed.
    ///
    /// May be [`None`] if the platform does not support pressure sensitivity.
    /// This feature is only available on **iOS** 9.0+ and **Windows** 8+.
    pub force: Option<ForceTouch>,
    /// The unique identifier of the finger.
    pub id: u64,
}

/// A force description of a [`Touch`](crate::touch::Touch) input.
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// A phase of a [`TouchInput`](crate::touch::TouchInput).
///
/// ## Usage
///
/// It is used to describe the phase of the touch input that is currently active.
/// This includes a phase that indicates that a touch input has started or ended,
/// or that a finger has moved. There is also a cancelled phase that indicates that
/// the system cancelled the tracking of the finger.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchPhase {
    /// A finger started to touch the touchscreen.
    Started,
    /// A finger moved over the touchscreen.
    Moved,
    /// A finger stopped touching the touchscreen.
    Ended,
    /// The system cancelled the tracking of the finger.
    ///
    /// This occurs when the window loses focus, or on iOS if the user moves the
    /// device against their face.
    Cancelled,
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
/// The resource is updated inside of the [`touch_screen_input_system`](crate::touch::touch_screen_input_system).
#[derive(Debug, Clone, Default)]
pub struct Touches {
    /// A collection of every [`Touch`] that is currently being pressed.
    pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got pressed.
    just_pressed: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got released.
    just_released: HashMap<u64, Touch>,
    /// A collection of every [`Touch`] that just got cancelled.
    just_cancelled: HashMap<u64, Touch>,
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

    /// Returns `true` if the input corresponding to the `id` has just been pressed.
    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains_key(&id)
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

    /// An iterator visiting every just released [`Touch`] input in arbitrary order.
    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> {
        self.just_released.values()
    }

    /// Checks if any touch input was just cancelled.
    pub fn any_just_cancelled(&self) -> bool {
        !self.just_cancelled.is_empty()
    }

    /// Returns `true` if the input corresponding to the `id` has just been cancelled.
    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains_key(&id)
    }

    /// An iterator visiting every just cancelled [`Touch`] input in arbitrary order.
    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> {
        self.just_cancelled.values()
    }

    /// Retrieves the position of the first currently pressed touch, if any
    pub fn first_pressed_position(&self) -> Option<Vec2> {
        self.pressed.values().next().map(|t| t.position)
    }

    /// Processes a [`TouchInput`] event by updating the `pressed`, `just_pressed`,
    /// `just_released`, and `just_cancelled` collections.
    fn process_touch_event(&mut self, event: &TouchInput) {
        match event.phase {
            TouchPhase::Started => {
                self.pressed.insert(event.id, event.into());
                self.just_pressed.insert(event.id, event.into());
            }
            TouchPhase::Moved => {
                if let Some(mut new_touch) = self.pressed.get(&event.id).cloned() {
                    new_touch.previous_position = new_touch.position;
                    new_touch.previous_force = new_touch.force;
                    new_touch.position = event.position;
                    new_touch.force = event.force;
                    self.pressed.insert(event.id, new_touch);
                }
            }
            TouchPhase::Ended => {
                self.just_released.insert(event.id, event.into());
                self.pressed.remove_entry(&event.id);
            }
            TouchPhase::Cancelled => {
                self.just_cancelled.insert(event.id, event.into());
                self.pressed.remove_entry(&event.id);
            }
        };
    }

    /// Clears the `just_pressed`, `just_released`, and `just_cancelled` collections.
    ///
    /// This is not clearing the `pressed` collection, because it could incorrectly mark
    /// a touch input as not pressed eventhough it is pressed. This could happen if the
    /// touch input is not moving for a single frame and would therefore be marked as
    /// not pressed, because this function is called on every single frame no matter
    /// if there was an event or not.
    fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_cancelled.clear();
    }
}

/// Updates the [`Touches`] resource with the latest [`TouchInput`] events.
///
/// ## Differences
///
/// The main difference between the [`TouchInput`] event and the [`Touches`] resource is that
/// the latter has convenient functions like [`Touches::just_pressed`] and [`Touches::just_released`].
pub fn touch_screen_input_system(
    mut touch_state: ResMut<Touches>,
    mut touch_input_events: EventReader<TouchInput>,
) {
    touch_state.update();

    for event in touch_input_events.iter() {
        touch_state.process_touch_event(event);
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn touch_update() {
        use crate::{touch::Touch, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = Touch {
            id: 4,
            start_position: Vec2::new(0.0, 0.0),
            start_force: None,
            previous_position: Vec2::new(0.0, 0.0),
            previous_force: None,
            position: Vec2::new(0.0, 0.0),
            force: None,
        };

        // Add a touch to `just_pressed`, 'just_released', and 'just cancelled'

        touches.just_pressed.insert(4, touch_event);
        touches.just_released.insert(4, touch_event);
        touches.just_cancelled.insert(4, touch_event);

        touches.update();

        // Verify that all the `just_x` maps are cleared
        assert!(touches.just_pressed.is_empty());
        assert!(touches.just_released.is_empty());
        assert!(touches.just_cancelled.is_empty());
    }

    #[test]
    fn touch_process() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        // Test adding a `TouchPhase::Started`

        let touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::new(4.0, 4.0),
            force: None,
            id: 4,
        };

        touches.update();
        touches.process_touch_event(&touch_event);

        assert!(touches.pressed.get(&touch_event.id).is_some());
        assert!(touches.just_pressed.get(&touch_event.id).is_some());

        // Test adding a `TouchPhase::Moved`

        let moved_touch_event = TouchInput {
            phase: TouchPhase::Moved,
            position: Vec2::new(5.0, 5.0),
            force: None,
            id: touch_event.id,
        };

        touches.update();
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
            phase: TouchPhase::Cancelled,
            position: Vec2::new(1.0, 1.0),
            force: None,
            id: touch_event.id,
        };

        touches.update();
        touches.process_touch_event(&cancel_touch_event);

        assert!(touches.just_cancelled.get(&cancel_touch_event.id).is_some());
        assert!(touches.pressed.get(&cancel_touch_event.id).is_none());

        // Test ending an event

        let end_touch_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::new(4.0, 4.0),
            force: None,
            id: 4,
        };

        touches.update();
        touches.process_touch_event(&touch_event);
        touches.process_touch_event(&end_touch_event);

        assert!(touches.just_released.get(&touch_event.id).is_some());
        assert!(touches.pressed.get(&touch_event.id).is_none());
    }

    #[test]
    fn touch_pressed() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Started,
            position: Vec2::new(4.0, 4.0),
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.get_pressed(touch_event.id).is_some());
        assert!(touches.just_pressed(touch_event.id));
        assert_eq!(touches.iter().count(), 1);
    }

    #[test]
    fn touch_released() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Ended,
            position: Vec2::new(4.0, 4.0),
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.get_released(touch_event.id).is_some());
        assert!(touches.just_released(touch_event.id));
        assert_eq!(touches.iter_just_released().count(), 1);
    }

    #[test]
    fn touch_cancelled() {
        use crate::{touch::TouchPhase, TouchInput, Touches};
        use bevy_math::Vec2;

        let mut touches = Touches::default();

        let touch_event = TouchInput {
            phase: TouchPhase::Cancelled,
            position: Vec2::new(4.0, 4.0),
            force: None,
            id: 4,
        };

        // Register the touch and test that it was registered correctly
        touches.process_touch_event(&touch_event);

        assert!(touches.just_cancelled(touch_event.id));
        assert_eq!(touches.iter_just_cancelled().count(), 1);
    }
}
