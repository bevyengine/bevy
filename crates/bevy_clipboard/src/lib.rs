//! This crate provides a platform-agnostic interface for accessing the clipboard

extern crate alloc;

pub use clipboard::*;

/// Clipboard plugin
#[derive(Default)]
pub struct ClipboardPlugin;

impl bevy_app::Plugin for ClipboardPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<Clipboard>();
    }
}

#[cfg(windows)]
mod clipboard {
    use bevy_ecs::resource::Resource;

    use crate::ClipboardError;

    /// Resource providing access to the clipboard
    #[derive(Resource, Default)]
    pub struct Clipboard;

    impl Clipboard {
        /// Fetches UTF-8 text from the clipboard and returns it.
        ///
        /// # Errors
        ///
        /// Returns error if clipboard is empty or contents are not UTF-8 text.
        pub fn get_text(&mut self) -> Result<String, ClipboardError> {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.get_text())
                .map_err(ClipboardError::from)
        }

        /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
        ///
        /// # Errors
        ///
        /// Returns error if `text` failed to be stored on the clipboard.
        pub fn set_text<'a, T: Into<alloc::borrow::Cow<'a, str>>>(
            &mut self,
            text: T,
        ) -> Result<(), ClipboardError> {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(text))
                .map_err(ClipboardError::from)
        }
    }
}

#[cfg(unix)]
mod clipboard {
    use crate::ClipboardError;
    use bevy_ecs::resource::Resource;

    /// Resource providing access to the clipboard
    #[derive(Resource)]
    pub struct Clipboard(Option<arboard::Clipboard>);

    impl Default for Clipboard {
        fn default() -> Self {
            Self(arboard::Clipboard::new().ok())
        }
    }

    impl Clipboard {
        /// Fetches UTF-8 text from the clipboard and returns it.
        ///
        /// # Errors
        ///
        /// Returns error if clipboard is empty or contents are not UTF-8 text.
        pub fn get_text(&mut self) -> Result<String, ClipboardError> {
            if let Some(clipboard) = self.0.as_mut() {
                clipboard.get_text().map_err(ClipboardError::from)
            } else {
                Err(ClipboardError::ClipboardNotSupported)
            }
        }

        /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
        ///
        /// # Errors
        ///
        /// Returns error if `text` failed to be stored on the clipboard.
        pub fn set_text<'a, T: Into<alloc::borrow::Cow<'a, str>>>(
            &mut self,
            text: T,
        ) -> Result<(), ClipboardError> {
            if let Some(clipboard) = self.0.as_mut() {
                clipboard.set_text(text).map_err(ClipboardError::from)
            } else {
                Err(arboard::Error::ClipboardNotSupported)
            }
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod clipboard {
    use crate::ClipboardError;
    use bevy_ecs::resource::Resource;

    /// Resource providing access to the clipboard
    #[derive(Resource, Default)]
    pub struct Clipboard;

    impl Clipboard {
        /// Fetches UTF-8 text from the clipboard and returns it.
        ///
        /// # Errors
        ///
        /// Returns error if clipboard is empty or contents are not UTF-8 text.
        pub fn get_text(&mut self) -> Result<String, ClipboardError> {
            Err(ClipboardError::ClipboardNotSupported)
        }

        /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
        ///
        /// # Errors
        ///
        /// Returns error if `text` failed to be stored on the clipboard.
        pub fn set_text<'a, T: Into<alloc::borrow::Cow<'a, str>>>(
            &mut self,
            text: T,
        ) -> Result<(), ClipboardError> {
            Err(ClipboardError::ClipboardNotSupported)
        }
    }
}

/// An error that might happen during a clipboard operation.
///
/// Note that both the `Display` and the `Debug` trait is implemented for this type in such a way
/// that they give a short human-readable description of the error; however the documentation
/// gives a more detailed explanation for each error kind.
///
/// Copied from `arboard::Error`
#[non_exhaustive]
pub enum ClipboardError {
    /// The clipboard contents were not available in the requested format.
    /// This could either be due to the clipboard being empty or the clipboard contents having
    /// an incompatible format to the requested one (eg when calling `get_image` on text)
    ContentNotAvailable,

    /// The selected clipboard is not supported by the current configuration (system and/or environment).
    ///
    /// This can be caused by a few conditions:
    /// - Using the Primary clipboard with an older Wayland compositor (that doesn't support version 2)
    /// - Using the Secondary clipboard on Wayland
    ClipboardNotSupported,

    /// The native clipboard is not accessible due to being held by an other party.
    ///
    /// This "other party" could be a different process or it could be within
    /// the same program. So for example you may get this error when trying
    /// to interact with the clipboard from multiple threads at once.
    ///
    /// Note that it's OK to have multiple `Clipboard` instances. The underlying
    /// implementation will make sure that the native clipboard is only
    /// opened for transferring data and then closed as soon as possible.
    ClipboardOccupied,

    /// The image or the text that was about the be transferred to/from the clipboard could not be
    /// converted to the appropriate format.
    ConversionFailure,

    /// Any error that doesn't fit the other error types.
    ///
    /// The `description` field is only meant to help the developer and should not be relied on as a
    /// means to identify an error case during runtime.
    Unknown {
        /// String describing the error
        description: String,
    },
}

#[cfg(any(windows, unix))]
impl From<arboard::Error> for ClipboardError {
    fn from(value: arboard::Error) -> Self {
        match value {
            arboard::Error::ContentNotAvailable => ClipboardError::ContentNotAvailable,
            arboard::Error::ClipboardNotSupported => ClipboardError::ClipboardNotSupported,
            arboard::Error::ClipboardOccupied => ClipboardError::ClipboardOccupied,
            arboard::Error::ConversionFailure => ClipboardError::ConversionFailure,
            arboard::Error::Unknown { description } => ClipboardError::Unknown { description },
            _ => ClipboardError::Unknown {
                description: "".to_owned(),
            },
        }
    }
}
