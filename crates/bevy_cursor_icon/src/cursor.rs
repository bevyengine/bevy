use bevy_ecs::component::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;

#[cfg(feature = "custom_cursor")]
use crate::custom_cursor::CustomCursor;

use crate::SystemCursorIcon;

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
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

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
/// A custom cursor created from a URL.
#[derive(Debug, Clone, Default, Reflect, PartialEq, Eq, Hash)]
#[reflect(Debug, Default, Hash, PartialEq, Clone)]
pub struct CustomCursorUrl {
    /// Web URL to an image to use as the cursor. PNGs are preferred. Cursor
    /// creation can fail if the image is invalid or not reachable.
    pub url: String,
    /// X and Y coordinates of the hotspot in pixels. The hotspot must be within
    /// the image bounds.
    pub hotspot: (u16, u16),
}
