//! Gamepad input module

pub mod axis;
pub mod button;
pub mod device;
pub mod event;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{axis::prelude::*, button::prelude::*, device::prelude::*, event::prelude::*};
}
