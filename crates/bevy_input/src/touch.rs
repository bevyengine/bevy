use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_utils::{HashMap, HashSet};

/// A touch input event
#[derive(Debug, Clone)]
pub struct TouchInput {
    pub phase: TouchPhase,
    pub position: Vec2,
    ///
    /// ## Platform-specific
    ///
    /// Unique identifier of a finger.
    pub id: u64,
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

#[derive(Debug, Clone)]
pub struct Touch {
    pub id: u64,
    pub start_position: Vec2,
    pub previous_position: Vec2,
    pub position: Vec2,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }
}

#[derive(Default)]
pub struct Touches {
    active_touches: HashMap<u64, Touch>,
    just_pressed: HashSet<u64>,
    just_released: HashSet<u64>,
    just_cancelled: HashSet<u64>,
}

impl Touches {
    pub fn iter(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.active_touches.values()
    }

    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains(&id)
    }

    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_pressed
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }

    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains(&id)
    }

    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_released
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }

    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains(&id)
    }

    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_cancelled
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }
}

/// Updates the Touches resource with the latest TouchInput events
pub fn touch_screen_input_system(
    mut state: Local<TouchSystemState>,
    mut touch_state: ResMut<Touches>,
    touch_input_events: Res<Events<TouchInput>>,
) {
    touch_state.just_pressed.clear();

    let released_touch_ids: HashSet<_> = touch_state.just_released.iter().cloned().collect();
    let cancelled_touch_ids: HashSet<_> = touch_state.just_released.iter().cloned().collect();

    touch_state.just_released.clear();
    touch_state.just_cancelled.clear();

    for released_id in released_touch_ids {
        touch_state.active_touches.remove(&released_id);
    }

    for cancelled_id in cancelled_touch_ids {
        touch_state.active_touches.remove(&cancelled_id);
    }

    for event in state.touch_event_reader.iter(&touch_input_events) {
        let active_touch = touch_state.active_touches.get(&event.id);
        match event.phase {
            TouchPhase::Started => {
                touch_state.active_touches.insert(
                    event.id,
                    Touch {
                        id: event.id,
                        start_position: event.position,
                        previous_position: event.position,
                        position: event.position,
                    },
                );
                touch_state.just_pressed.insert(event.id);
            }
            TouchPhase::Moved => {
                let old_touch = active_touch.unwrap();
                let mut new_touch = old_touch.clone();
                new_touch.previous_position = new_touch.position;
                new_touch.position = event.position;
                touch_state.active_touches.insert(event.id, new_touch);
            }
            TouchPhase::Ended => {
                touch_state.just_released.insert(event.id);
            }
            TouchPhase::Cancelled => {
                touch_state.just_cancelled.insert(event.id);
            }
        };
    }
}
