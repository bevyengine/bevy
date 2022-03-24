pub mod axis;
pub mod button;
pub mod event;
pub mod gamepads;
pub mod settings;
pub mod system;
pub mod types;

pub use crate::gamepad::{
    axis::*, button::*, event::*, gamepads::*, settings::*, system::*, types::*,
};
