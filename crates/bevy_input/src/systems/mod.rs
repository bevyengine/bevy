//! Bevy input systems module.
//! Provides convenience systems for end users

pub mod exit_on_esc;
pub mod gamepad_events;
pub mod keyboard_input;
pub mod mouse_button;
pub mod touch_input;

pub use exit_on_esc::exit_on_esc_system;
pub use gamepad_events::gamepad_event_system;
pub use keyboard_input::keyboard_input_system;
pub use mouse_button::mouse_button_input_system;
pub use touch_input::touch_screen_input_system;
