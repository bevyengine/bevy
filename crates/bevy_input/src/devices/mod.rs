pub mod gamepad;
pub mod keyboard;
pub mod mouse;
pub mod touch;

pub use prelude::*;

pub mod prelude {
    pub use super::{
        gamepad::prelude::*, keyboard::prelude::*, mouse::prelude::*, touch::prelude::*,
    };
}
