mod cursor;
mod system_cursor;

#[cfg(feature = "custom_cursor")]
mod custom_cursor;

use bevy_app::{App, Plugin};
pub use cursor::*;
pub use system_cursor::SystemCursorIcon;

#[cfg(feature = "custom_cursor")]
pub use custom_cursor::*;

pub mod prelude {
    pub use crate::system_cursor::SystemCursorIcon;
}

#[derive(Default)]
pub struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CursorIcon>();
    }
}
