use crate::touch::{ForceTouch, TouchInput, TouchPhase};
use bevy_math::Vec2;
use bevy_utils::HashMap;

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

    pub(crate) fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.just_cancelled.clear();
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
