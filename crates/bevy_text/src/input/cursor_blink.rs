use core::time::Duration;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, prelude::ReflectComponent, resource::Resource};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

/// The time taken for the cursor to blink
#[derive(Resource)]
pub struct TextCursorBlinkInterval(pub Duration);

impl Default for TextCursorBlinkInterval {
    fn default() -> Self {
        Self(Duration::from_secs_f32(0.5))
    }
}

/// Optional component to control cursor blink behavior
#[derive(Default, Component, Clone, Debug, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Debug)]
pub struct CursorBlink {
    /// Controls cursor blinking.
    /// If the value is greater than the `blink_interval` in `TextCursorStyle` then the cursor
    /// is not displayed.
    /// The timer is reset when a `TextEdit` is applied.
    pub cursor_blink_timer: f32,
}
