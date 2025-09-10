//! This crate provides a platform-agnostic interface for accessing the clipboard

extern crate alloc;

use bevy_ecs::resource::Resource;

#[cfg(target_arch = "wasm32")]
use {alloc::sync::Arc, bevy_platform::sync::Mutex, wasm_bindgen_futures::JsFuture};

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
#[derive(Debug, Clone)]
pub enum ClipboardRead {
    /// The clipboard contents are ready to be accessed.
    Ready(Result<String, ClipboardError>),
    #[cfg(target_arch = "wasm32")]
    /// The clipboard contents are being fetched asynchronously.
    Pending(Arc<Mutex<Option<Result<String, ClipboardError>>>>),
}

impl ClipboardRead {
    /// The result of an attempt to read from the clipboard, once ready.
    /// If the result is still pending, returns `None`.
    pub fn poll_result(&mut self) -> Option<Result<String, ClipboardError>> {
        match self {
            #[cfg(target_arch = "wasm32")]
            Self::Pending(shared) => {
                if let Some(contents) = shared.lock().ok().and_then(|mut inner| inner.take()) {
                    *self = Self::Ready(Err(ClipboardError::ContentTaken));
                    Some(contents)
                } else {
                    None
                }
            }
            Self::Ready(inner) => {
                Some(core::mem::replace(inner, Err(ClipboardError::ContentTaken)))
            }
        }
    }
}

/// Resource providing access to the clipboard
#[derive(Resource)]
pub struct Clipboard {
    /// Use arboard to access host clipboard
    #[cfg(unix)]
    host_clipboard: Option<arboard::Clipboard>,

    /// Fallback basic clipboard implementation that only works within the bevy app.
    local_clipboard: Option<String>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            #[cfg(unix)]
            host_clipboard: arboard::Clipboard::new().ok(),
            local_clipboard: None,
        }
    }
}

// TODO: not entirely consistent with `local_clipboard` usage, fix this
impl Clipboard {
    /// Fetches UTF-8 text from the clipboard and returns it via a `ClipboardRead`.
    ///
    /// On Windows and Unix `ClipboardRead`s are completed instantly, on wasm32 the result is fetched asynchronously.
    pub fn fetch_text(&mut self) -> ClipboardRead {
        #[cfg(unix)]
        {
            ClipboardRead::Ready(if let Some(clipboard) = self.host_clipboard.as_mut() {
                clipboard.get_text().map_err(ClipboardError::from)
            } else {
                self.local_clipboard
                    .clone()
                    .ok_or(ClipboardError::ContentNotAvailable)
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
                ClipboardRead::Ready(
                    self.local_clipboard
                        .clone()
                        .ok_or(ClipboardError::ContentNotAvailable),
                )
            }
        }

        #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
        {
            ClipboardRead::Ready(
                self.local_clipboard
                    .clone()
                    .ok_or(ClipboardError::ContentNotAvailable),
            )
        }
    }

    /// Asynchronously retrieves UTF-8 text from the system clipboard.
    pub async fn fetch_text_async(&mut self) -> Result<String, ClipboardError> {
        #[cfg(unix)]
        {
            if let Some(clipboard) = self.host_clipboard.as_mut() {
                clipboard.get_text().map_err(ClipboardError::from)
            } else {
                self.local_clipboard
                    .clone()
                    .ok_or(ClipboardError::ContentNotAvailable)
            }
        }

        #[cfg(windows)]
        {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.get_text())
                .map_err(ClipboardError::from)
        }

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;

            let clipboard = web_sys::window()
                .and_then(|w| Some(w.navigator().clipboard()))
                .ok_or(ClipboardError::ClipboardNotSupported)?;

            let result = JsFuture::from(clipboard.read_text()).await;
            match result {
                Ok(val) => val.as_string().ok_or(ClipboardError::ConversionFailure),
                Err(_) => Err(ClipboardError::ContentNotAvailable),
            }
        }

        #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
        {
            self.local_clipboard
                .clone()
                .ok_or(ClipboardError::ContentNotAvailable)
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
            if let Some(clipboard) = self.host_clipboard.as_mut() {
                clipboard.set_text(text).map_err(ClipboardError::from)
            } else {
                let k: alloc::borrow::Cow<'a, str> = text.into();
                let j: String = k.into();
                self.local_clipboard = Some(j);
                Ok(())
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
                let text = text.into().to_string();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = JsFuture::from(clipboard.write_text(&text)).await;
                });
                Ok(())
            } else {
                let k: alloc::borrow::Cow<'a, str> = text.into();
                let j: String = k.into();
                self.local_clipboard = Some(j);
                Ok(())
            }
        }

        #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
        {
            let k: alloc::borrow::Cow<'a, str> = text.into();
            let j: String = k.into();
            self.local_clipboard = Some(j);
            Ok(())
        }
    }
}

/// An error that might happen during a clipboard operation.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ClipboardError {
    /// Clipboard contents were unavailable or not in the expected format.
    ContentNotAvailable,

    /// Clipboard access is temporarily locked by another process or thread.
    ClipboardOccupied,

    /// The data could not be converted to or from the required format.
    ConversionFailure,

    /// The clipboard content was already taken from the `ClipboardRead`.
    ContentTaken,

    /// An unknown error
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
            arboard::Error::ClipboardOccupied => ClipboardError::ClipboardOccupied,
            arboard::Error::ConversionFailure => ClipboardError::ConversionFailure,
            arboard::Error::Unknown { description } => ClipboardError::Unknown { description },
            _ => ClipboardError::Unknown {
                description: "Unknown arboard error variant".to_owned(),
            },
        }
    }
}
