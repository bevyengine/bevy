pub mod core;
pub mod gamepad;
pub mod keyboard;
pub mod mouse;
pub mod touch;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use crate::{
        core::prelude::*, gamepad::prelude::*, keyboard::prelude::*, mouse::prelude::*,
        touch::prelude::*,
    };
}
