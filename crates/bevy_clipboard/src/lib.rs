//! This crate provides a platform-agnostic interface for accessing the clipboard

extern crate alloc;

use bevy_ecs::resource::Resource;

use {alloc::sync::Arc, bevy_platform::sync::Mutex};

/// The clipboard prelude
pub mod prelude {
    pub use crate::{Clipboard, ClipboardRead};
}

/// Clipboard plugin
#[derive(Default)]
pub struct ClipboardPlugin;

impl bevy_app::Plugin for ClipboardPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<Clipboard>();
    }
}

/// Represents an attempt to read from the clipboard.
///
/// On desktop targets the result is available immediately.
/// On wasm32 the result is fetched asynchronously.
#[derive(Debug)]
pub enum ClipboardRead {
    /// The clipboard contents are ready to be accessed.
    Ready(Result<String, ClipboardError>),
    /// The clipboard contents are being fetched asynchronously.
    Pending(Arc<Mutex<Option<Result<String, ClipboardError>>>>),
}

impl ClipboardRead {
    /// The result of an attempt to read from the clipboard, if it is ready.
    /// If the result is still pending returns `None`.
    pub fn poll_result(&mut self) -> Option<Result<String, ClipboardError>> {
        match self {
            Self::Pending(shared) => {
                if let Some(contents) = shared.lock().ok().and_then(|mut inner| inner.take()) {
                    *self = Self::Ready(Err(ClipboardError::ContentTaken));
                    Some(contents)
                } else {
                    None
                }
            }
            Self::Ready(inner) => Some(std::mem::replace(inner, Err(ClipboardError::ContentTaken))),
        }
    }
}

/// Resource providing access to the clipboard
#[cfg(unix)]
#[derive(Resource)]
pub struct Clipboard(Option<arboard::Clipboard>);

#[cfg(unix)]
impl Default for Clipboard {
    fn default() -> Self {
        {
            Self(arboard::Clipboard::new().ok())
        }
    }
}

/// Resource providing access to the clipboard
#[cfg(not(unix))]
#[derive(Resource, Default)]
pub struct Clipboard;

impl Clipboard {
    /// Fetches UTF-8 text from the clipboard.
    ///
    /// On Windows and Unix this completed immediately, while
    ///
    /// # Errors
    ///
    /// Returns error if clipboard is empty or contents are not UTF-8 text.
    pub fn fetch_text(&mut self) -> ClipboardRead {
        #[cfg(unix)]
        {
            ClipboardRead::Ready(if let Some(clipboard) = self.0.as_mut() {
                clipboard.get_text().map_err(ClipboardError::from)
            } else {
                Err(ClipboardError::ClipboardNotSupported)
            })
        }

        #[cfg(windows)]
        {
            ClipboardRead::Ready(
                arboard::Clipboard::new()
                    .and_then(|mut clipboard| clipboard.get_text())
                    .map_err(ClipboardError::from),
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
                let shared = Arc::new(Mutex::new(None));
                let shared_clone = shared.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let text = JsFuture::from(clipboard.read_text()).await;
                    let text = match text {
                        Ok(text) => text.as_string().ok_or(ClipboardError::ConversionFailure),
                        Err(_) => Err(ClipboardError::ContentNotAvailable),
                    };
                    shared.lock().unwrap().replace(text);
                });
                ClipboardRead::Pending(shared_clone)
            } else {
                ClipboardRead::Ready(Err(ClipboardError::ClipboardNotSupported))
            }
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
        #[cfg(unix)]
        {
            if let Some(clipboard) = self.0.as_mut() {
                clipboard.set_text(text).map_err(ClipboardError::from)
            } else {
                Err(ClipboardError::ClipboardNotSupported)
            }
        }

        #[cfg(windows)]
        {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(text))
                .map_err(ClipboardError::from)
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = JsFuture::from(clipboard.write_text(&text)).await;
                });
                Ok(())
            } else {
                Err(ClipboardError::ClipboardNotSupported)
            }
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
#[derive(Debug, Clone)]
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

    /// The clipboard content was already taken from the `ClipboardRead`.
    ContentTaken,

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
                description: "Unknown arboard error variant".to_owned(),
            },
        }
    }
}
