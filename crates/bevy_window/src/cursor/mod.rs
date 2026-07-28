//! Cursor handling for windows.
//!
//! Note: [`CursorSystems`] lives in `bevy_window` rather than `bevy_winit` so
//! that systems which need to order themselves around cursor updates don't have
//! to depend on `bevy_winit` just for this set. The system set is intended to
//! be backend-agnostic and consumable by any windowing backend (`bevy_winit`,
//! or a future one).

#[cfg(feature = "custom_cursor")]
mod custom_cursor;
mod system_cursor;

#[cfg(feature = "custom_cursor")]
pub use custom_cursor::*;
pub use system_cursor::*;

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{component::Component, schedule::SystemSet};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

#[cfg(feature = "custom_cursor")]
pub use crate::cursor::{CustomCursor, CustomCursorImage};

/// System sets for cursor-related systems.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet, Reflect)]
pub enum CursorSystems {
    /// Reads changes to [`CursorIcon`] and queues the corresponding cursor to
    /// be applied to the window by the winit event loop.
    ///
    /// Order your systems before this set to set [`CursorIcon`] and have it
    /// take effect on the same frame.
    Update,
}

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Debug, Default, PartialEq, Clone)
)]
pub enum CursorIcon {
    #[cfg(feature = "custom_cursor")]
    /// Custom cursor image.
    Custom(CustomCursor),
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::System(Default::default())
    }
}

impl From<SystemCursorIcon> for CursorIcon {
    fn from(icon: SystemCursorIcon) -> Self {
        CursorIcon::System(icon)
    }
}

impl CursorIcon {
    /// Returns the system cursor icon if this is a system cursor.
    pub fn as_system(&self) -> Option<&SystemCursorIcon> {
        #[cfg(feature = "custom_cursor")]
        {
            if let CursorIcon::System(icon) = self {
                Some(icon)
            } else {
                None
            }
        }
        #[cfg(not(feature = "custom_cursor"))]
        {
            let CursorIcon::System(icon) = self;
            Some(icon)
        }
    }
}
