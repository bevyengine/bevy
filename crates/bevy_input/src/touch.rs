use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_utils::HashMap;

/// Represents a touch event
///
/// Every time the user touches the screen, a new `Start` event with an unique
/// identifier for the finger is generated. When the finger is lifted, an `End`
/// event is generated with the same finger id.
///
/// After a `Start` event has been emitted, there may be zero or more `Move`
/// events when the finger is moved or the touch pressure changes.
///
/// The finger id may be reused by the system after an `End` event. The user
/// should assume that a new `Start` event received with the same id has nothing
/// to do with the old finger and is a new finger.
///
/// A `Cancelled` event is emitted when the system has canceled tracking this
/// touch, such as when the window loses focus, or on iOS if the user moves the
/// device against their face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchInput {
    pub phase: TouchPhase,
    pub position: Vec2,
    /// Describes how hard the screen was pressed. May be `None` if the platform
    /// does not support pressure sensitivity.
    ///
    /// ## Platform-specific
    ///
    /// - Only available on **iOS** 9.0+ and **Windows** 8+.
    pub force: Option<ForceTouch>,
    /// Unique identifier of a finger.
    pub id: u64,
}

/// Describes the force of a touch event
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
    /// press really really hard, or not hard at all, depending on the device.
    Normalized(f64),
}

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Default)]
pub struct TouchSystemState {
    touch_event_reader: EventReader<TouchInput>,
}

#[derive(Debug, Clone, Copy)]
pub struct Touch {
    id: u64,
    start_position: Vec2,
    start_force: Option<ForceTouch>,
    previous_position: Vec2,
    previous_force: Option<ForceTouch>,
    position: Vec2,
    force: Option<ForceTouch>,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    #[inline]
    pub fn start_position(&self) -> Vec2 {
        self.start_position
    }

    #[inline]
    pub fn start_force(&self) -> Option<ForceTouch> {
        self.start_force
    }

    #[inline]
    pub fn previous_position(&self) -> Vec2 {
        self.previous_position
    }

    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

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

#[derive(Debug, Clone, Default)]
pub struct Touches {
    pressed: HashMap<u64, Touch>,
    just_pressed: HashMap<u64, Touch>,
    just_released: HashMap<u64, Touch>,
    just_cancelled: HashMap<u64, Touch>,
}

impl Touches {
    pub fn iter(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.pressed.values()
    }

    pub fn get_pressed(&self, id: u64) -> Option<&Touch> {
        self.pressed.get(&id)
    }

    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains_key(&id)
    }

    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> {
        self.just_pressed.values()
    }

    pub fn get_released(&self, id: u64) -> Option<&Touch> {
        self.just_released.get(&id)
    }

    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains_key(&id)
    }

    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> {
        self.just_released.values()
    }

    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains_key(&id)
    }

    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> {
        self.just_cancelled.values()
    }

    fn process_touch_event(&mut self, event: &TouchInput) {
        match event.phase {
            TouchPhase::Started => {
                self.pressed.insert(event.id, event.into());
                self.just_pressed.insert(event.id, event.into());
            }
            TouchPhase::Moved => {
                let mut new_touch = self.pressed.get(&event.id).cloned().unwrap();
                new_touch.previous_position = new_touch.position;
                new_touch.previous_force = new_touch.force;
                new_touch.position = event.position;
                new_touch.force = event.force;
                self.pressed.insert(event.id, new_touch);
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

    fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_cancelled.clear();
    }
}

/// Updates the Touches resource with the latest TouchInput events
pub fn touch_screen_input_system(
    mut state: Local<TouchSystemState>,
    mut touch_state: ResMut<Touches>,
    touch_input_events: Res<Events<TouchInput>>,
) {
    touch_state.update();

    for event in state.touch_event_reader.iter(&touch_input_events) {
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
