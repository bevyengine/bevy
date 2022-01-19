use crate::touch::{ForceTouch, TouchInput, TouchPhase};
use bevy_math::Vec2;
use bevy_utils::HashMap;

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

    /// Returns the `id` of the [`Touch`].
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the `start_position` of the [`Touch`].
    #[inline]
    pub fn start_position(&self) -> Vec2 {
        self.start_position
    }

    /// Returns the `start_force` of the [`Touch`].
    #[inline]
    pub fn start_force(&self) -> Option<ForceTouch> {
        self.start_force
    }

    /// Returns the `previous_position` of the [`Touch`].
    #[inline]
    pub fn previous_position(&self) -> Vec2 {
        self.previous_position
    }

    /// Returns the `previous_force` of the [`Touch`].
    #[inline]
    pub fn previous_force(&self) -> Option<ForceTouch> {
        self.previous_force
    }

    /// Returns the current `position` of the [`Touch`].
    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// Returns the current `force` of the [`Touch`].
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
/// ## Access
///
/// To access the resource use one of the following:
/// - Non-mutable access of the touch inputs: `Res<Touches>`
/// - Mutable access of the touch inputs: `ResMut<Touches>`
///
/// ## Updating
///
/// The resource is updated inside of the [`touch_screen_input_system`](crate::touch::touch_screen_input_system).
#[derive(Debug, Clone, Default)]
pub struct Touches {
    /// A collection of every button that is currently being pressed.
    pressed: HashMap<u64, Touch>,
    /// A collection of every button that just got pressed.
    just_pressed: HashMap<u64, Touch>,
    /// A collection of every button that just got released.
    just_released: HashMap<u64, Touch>,
    /// A collection of every button that just got cancelled.
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

    /// Returns `true` if the input corresponding to the `id` has just been released.
    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains_key(&id)
    }

    /// An iterator visiting every just released [`Touch`] input in arbitrary order.
    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> {
        self.just_released.values()
    }

    /// Returns `true` if the input corresponding to the `id` has just been cancelled.
    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains_key(&id)
    }

    /// An iterator visiting every just cancelled [`Touch`] input in arbitrary order.
    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> {
        self.just_cancelled.values()
    }

    /// Processes a [`TouchInput`] event by updating the `pressed`, `just_pressed`,
    /// `just_released`, and `just_cancelled` collections.
    pub(crate) fn process_touch_event(&mut self, event: &TouchInput) {
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
    pub(crate) fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_cancelled.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::touch::{ForceTouch, Touch, TouchInput, TouchPhase, Touches};
    use bevy_math::Vec2;

    pub fn create_touch(id: u64) -> Touch {
        Touch {
            id,
            start_position: Vec2::new(1.0, 1.0),
            start_force: Some(ForceTouch::Normalized(1.0)),
            previous_position: Vec2::new(2.0, 2.0),
            previous_force: Some(ForceTouch::Normalized(2.0)),
            position: Vec2::new(3.0, 3.0),
            force: Some(ForceTouch::Normalized(3.0)),
        }
    }

    mod touch {
        use super::*;

        #[test]
        fn test_delta() {
            let touch = create_touch(1);
            assert_eq!(touch.delta(), Vec2::new(1.0, 1.0));
        }

        #[test]
        fn test_distance() {
            let touch = create_touch(1);
            assert_eq!(touch.distance(), Vec2::new(2.0, 2.0));
        }

        #[test]
        fn test_id() {
            let touch = create_touch(1);
            assert_eq!(touch.id(), touch.id);
        }

        #[test]
        fn test_start_position() {
            let touch = create_touch(1);
            assert_eq!(touch.start_position(), touch.start_position);
        }

        #[test]
        fn test_start_force() {
            let touch = create_touch(1);
            assert_eq!(touch.start_force(), touch.start_force);
        }

        #[test]
        fn test_previous_position() {
            let touch = create_touch(1);
            assert_eq!(touch.previous_position(), touch.previous_position);
        }

        #[test]
        fn test_previous_force() {
            let touch = create_touch(1);
            assert_eq!(touch.previous_force(), touch.previous_force);
        }

        #[test]
        fn test_position() {
            let touch = create_touch(1);
            assert_eq!(touch.position(), touch.position);
        }

        #[test]
        fn test_force() {
            let touch = create_touch(1);
            assert_eq!(touch.force(), touch.force);
        }
    }

    mod touches {
        use super::*;

        fn create_touches() -> Touches {
            let mut touches = Touches::default();

            // Pressed
            touches.pressed.insert(1, create_touch(1));
            touches.pressed.insert(2, create_touch(2));
            touches.pressed.insert(3, create_touch(3));

            // Just pressed
            touches.just_pressed.insert(4, create_touch(4));
            touches.just_pressed.insert(5, create_touch(5));
            touches.just_pressed.insert(6, create_touch(6));

            // Just released
            touches.just_released.insert(7, create_touch(7));
            touches.just_released.insert(8, create_touch(8));
            touches.just_released.insert(9, create_touch(9));

            // Just cancelled
            touches.just_cancelled.insert(10, create_touch(10));
            touches.just_cancelled.insert(11, create_touch(11));
            touches.just_cancelled.insert(12, create_touch(12));

            touches
        }

        #[test]
        fn test_iter() {
            let touches = create_touches();
            for touch in touches.iter() {
                assert!(touches.pressed.contains_key(&touch.id));
            }
        }

        #[test]
        fn test_get_pressed() {
            let touches = create_touches();
            assert_eq!(touches.get_pressed(1).unwrap().id, 1);
            assert_eq!(touches.get_pressed(2).unwrap().id, 2);
            assert_eq!(touches.get_pressed(3).unwrap().id, 3);
        }

        #[test]
        fn test_just_pressed() {
            let touches = create_touches();
            assert!(touches.just_pressed(4));
            assert!(touches.just_pressed(5));
            assert!(touches.just_pressed(6));
            assert!(!touches.just_pressed(100));
            assert!(!touches.just_pressed(101));
            assert!(!touches.just_pressed(102));
        }

        #[test]
        fn test_iter_just_pressed() {
            let touches = create_touches();
            for touch in touches.iter_just_pressed() {
                assert!(touches.just_pressed.contains_key(&touch.id));
            }
        }

        #[test]
        fn test_get_released() {
            let touches = create_touches();
            assert_eq!(touches.get_released(7).unwrap().id, 7);
            assert_eq!(touches.get_released(8).unwrap().id, 8);
            assert_eq!(touches.get_released(9).unwrap().id, 9);
        }

        #[test]
        fn test_just_released() {
            let touches = create_touches();
            assert!(touches.just_released(7));
            assert!(touches.just_released(8));
            assert!(touches.just_released(9));
            assert!(!touches.just_released(100));
            assert!(!touches.just_released(101));
            assert!(!touches.just_released(102));
        }

        #[test]
        fn test_iter_just_released() {
            let touches = create_touches();
            for touch in touches.iter_just_released() {
                assert!(touches.just_released.contains_key(&touch.id));
            }
        }

        #[test]
        fn test_just_cancelled() {
            let touches = create_touches();
            assert!(touches.just_cancelled(10));
            assert!(touches.just_cancelled(11));
            assert!(touches.just_cancelled(12));
            assert!(!touches.just_cancelled(100));
            assert!(!touches.just_cancelled(101));
            assert!(!touches.just_cancelled(102));
        }

        #[test]
        fn test_iter_just_cancelled() {
            let touches = create_touches();
            for touch in touches.iter_just_cancelled() {
                assert!(touches.just_cancelled.contains_key(&touch.id));
            }
        }

        #[test]
        fn test_process_touch_event() {
            // Started -> Moved -> Ended
            let mut touches = Touches::default();

            // Started
            let event = TouchInput {
                phase: TouchPhase::Started,
                position: Vec2::ZERO,
                force: None,
                id: 1000,
            };
            touches.process_touch_event(&event);
            assert!(touches.pressed.contains_key(&1000));
            assert!(touches.just_pressed.contains_key(&1000));

            // Moved
            let event = TouchInput {
                phase: TouchPhase::Moved,
                position: Vec2::new(1.0, 1.0),
                force: Some(ForceTouch::Normalized(1.0)),
                id: 1000,
            };
            touches.process_touch_event(&event);
            let touch = touches.pressed.get(&event.id).unwrap();
            assert_eq!(touch.previous_position, Vec2::ZERO);
            assert_eq!(touch.previous_force, None);
            assert_eq!(touch.position, Vec2::new(1.0, 1.0));
            assert_eq!(touch.force, Some(ForceTouch::Normalized(1.0)));

            // Ended
            let event = TouchInput {
                phase: TouchPhase::Ended,
                position: Vec2::ZERO,
                force: None,
                id: 1000,
            };
            touches.process_touch_event(&event);
            assert!(touches.just_released.contains_key(&1000));
            assert!(!touches.pressed.contains_key(&1000));

            // Started -> Moved -> Cancelled
            let mut touches = Touches::default();

            // Started
            let event = TouchInput {
                phase: TouchPhase::Started,
                position: Vec2::new(1.0, 1.0),
                force: Some(ForceTouch::Normalized(1.0)),
                id: 1000,
            };
            touches.process_touch_event(&event);
            assert!(touches.pressed.contains_key(&1000));
            assert!(touches.just_pressed.contains_key(&1000));

            // Moved
            let event = TouchInput {
                phase: TouchPhase::Moved,
                position: Vec2::new(2.0, 2.0),
                force: Some(ForceTouch::Normalized(2.0)),
                id: 1000,
            };
            touches.process_touch_event(&event);
            let touch = touches.pressed.get(&event.id).unwrap();
            assert_eq!(touch.previous_position, Vec2::new(1.0, 1.0));
            assert_eq!(touch.previous_force, Some(ForceTouch::Normalized(1.0)));
            assert_eq!(touch.position, Vec2::new(2.0, 2.0));
            assert_eq!(touch.force, Some(ForceTouch::Normalized(2.0)));

            // Cancelled
            let event = TouchInput {
                phase: TouchPhase::Cancelled,
                position: Vec2::ZERO,
                force: None,
                id: 1000,
            };
            touches.process_touch_event(&event);
            assert!(touches.just_cancelled.contains_key(&1000));
            assert!(!touches.pressed.contains_key(&1000));
        }

        #[test]
        fn test_update() {
            let mut touches = create_touches();
            touches.update();
            assert_eq!(touches.pressed.len(), 3);
            assert!(touches.just_pressed.is_empty());
            assert!(touches.just_released.is_empty());
            assert!(touches.just_cancelled.is_empty());
        }
    }
}
