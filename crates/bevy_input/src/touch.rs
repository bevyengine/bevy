use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_utils::HashMap;
use core::ops::DerefMut;

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
    pub id: u64,
    pub start_position: Vec2,
    pub start_force: Option<ForceTouch>,
    pub previous_position: Vec2,
    pub previous_force: Option<ForceTouch>,
    pub position: Vec2,
    pub force: Option<ForceTouch>,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
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

    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_pressed
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }

    pub fn get_released(&self, id: u64) -> Option<&Touch> {
        self.just_released.get(&id)
    }

    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains_key(&id)
    }

    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_released
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }

    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains_key(&id)
    }

    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_cancelled
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }
}

/// Updates the Touches resource with the latest TouchInput events
pub fn touch_screen_input_system(
    mut state: Local<TouchSystemState>,
    mut touch_state: ResMut<Touches>,
    touch_input_events: Res<Events<TouchInput>>,
) {
    let touch_state = touch_state.deref_mut();
    touch_state.just_pressed.clear();
    touch_state.just_released.clear();
    for event in state.touch_event_reader.iter(&touch_input_events) {
        match event.phase {
            TouchPhase::Started => {
                touch_state.pressed.insert(event.id, event.into());
                touch_state.just_pressed.insert(event.id, event.into());
            }
            TouchPhase::Moved => {
                let mut new_touch = touch_state.pressed.get(&event.id).cloned().unwrap();
                new_touch.previous_position = new_touch.position;
                new_touch.previous_force = new_touch.force;
                new_touch.position = event.position;
                new_touch.force = event.force;
                touch_state.pressed.insert(event.id, new_touch);
            }
            TouchPhase::Ended => {
                touch_state.just_released.insert(event.id, event.into());
                touch_state.pressed.remove_entry(&event.id);
            }
            TouchPhase::Cancelled => {
                touch_state.just_cancelled.insert(event.id, event.into());
                touch_state.pressed.remove_entry(&event.id);
            }
        };
    }
}
