//! This crate provides a platform-agnostic interface for accessing the clipboard.
//!
//! Read (and write) to the [`Clipboard`] resource to interact with the system clipboard.
//!
//! Note that this crate is deliberately low-level with minimal dependencies:
//! it does not provide any input integration for clipboard operations,
//! such as Ctrl+C/Ctrl+V support.
//!
//! This should be provided by other crates (or your own systems) which depend on `bevy_clipboard`,
//! such as `bevy_ui_widgets` in the case of text editing.
//!
//! `bevy_clipboard`'s primary advantage over using [`arboard`](https://crates.io/crates/arboard) directly is that
//! it provides a consistent API across all platforms, with a simple but robust fallback when `arboard`
//! is not available or clipboard permissions are not granted.
//!
//! ## Platform support
//!
//! On Android and iOS, `arboard` is not available and the `system_clipboard` feature has no
//! effect. The [`Clipboard`] resource still works, but reads and writes go to an in-process
//! buffer that is invisible to other applications and does not survive process exit.
//!
//! On Windows and Unix, clipboard operations are performed synchronously and results are
//! available immediately. On wasm32, results are accessed via [`ClipboardRead`], which can
//! be polled for completion.
//!
//! Images are supported on Windows and Unix when the `image` feature is enabled, which depends on `system_clipboard`.
//! Image support is not available on wasm32, Android, or iOS.

extern crate alloc;

use alloc::borrow::Cow;
#[cfg(feature = "image")]
use bevy_asset::RenderAssetUsages;
use bevy_ecs::resource::Resource;
#[cfg(feature = "image")]
use bevy_image::Image;
#[cfg(feature = "image")]
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
use {alloc::sync::Arc, bevy_platform::sync::Mutex};

/// Commonly used types and traits from `bevy_clipboard`.
pub mod prelude {
    pub use crate::{Clipboard, ClipboardPlugin, ClipboardRead};
}

/// Adds clipboard support to a Bevy app.
///
/// The [`Clipboard`] resource is your main entry point.
///
/// See the [crate docs](crate) for more details.
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
/// On web, the result is fetched asynchronously.
///
/// The generic `T` parameter represents the type of clipboard content that we are attempting to read,
/// which is `String` by default for text reads.
/// If the clipboard contents do not match this type,
/// the read will fail with a [`ClipboardError::ContentNotAvailable`]
/// or [`ClipboardError::ConversionFailure`] error.
///
/// ## Note on cloning
///
/// [`Clone`] on a [`ClipboardRead::Pending`] shares the underlying in-flight read, since
/// the inner state is held in an [`Arc`].
/// Only the first of the clones to successfully [`poll_result`](ClipboardRead::poll_result) will observe the value;
/// subsequent pollers will see `None` as if the read were still pending.
#[derive(Debug, Clone)]
pub enum ClipboardRead<T = String> {
    /// The clipboard contents are ready to be accessed.
    Ready(Result<T, ClipboardError>),
    /// The clipboard contents are being fetched asynchronously.
    ///
    /// The `Option` is `None` while the read is still pending, and becomes `Some` once the read completes with either success or error.
    /// `Some(Ok)` indicates a successful read with the clipboard contents, while `Some(Err)` indicates a failure to read the clipboard.
    Pending(Arc<Mutex<Option<Result<T, ClipboardError>>>>),
    /// The clipboard contents have already been taken by a previous call to [`ClipboardRead::poll_result`].
    Taken,
}

impl<T> ClipboardRead<T> {
    /// The result of an attempt to read from the clipboard, once ready.
    ///
    /// Returns `None` if the result is still pending or has already been taken.
    pub fn poll_result(&mut self) -> Option<Result<T, ClipboardError>> {
        match self {
            Self::Pending(shared) => {
                let contents = shared.lock().ok().and_then(|mut inner| inner.take())?;
                *self = Self::Taken;
                Some(contents)
            }
            Self::Ready(_) => {
                let Self::Ready(inner) = core::mem::replace(self, Self::Taken) else {
                    unreachable!()
                };
                Some(inner)
            }
            Self::Taken => None,
        }
    }
}

#[cfg(feature = "image")]
fn try_image_from_imagedata(image: arboard::ImageData<'static>) -> Result<Image, ClipboardError> {
    let size = Extent3d {
        width: u32::try_from(image.width).map_err(|_| ClipboardError::ConversionFailure)?,
        height: u32::try_from(image.height).map_err(|_| ClipboardError::ConversionFailure)?,
        depth_or_array_layers: 1,
    };
    Ok(Image::new(
        size,
        TextureDimension::D2,
        image.bytes.into_owned(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

#[cfg(feature = "image")]
fn try_imagedata_from_image(image: &Image) -> Result<arboard::ImageData<'_>, ClipboardError> {
    // arboard expects packed RGBA8.
    // We need to reject anything else: a same-size format like
    // Bgra8Unorm would pass the length check but produce corrupt colors.
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ) {
        return Err(ClipboardError::ConversionFailure);
    }

    let width = image.width() as usize;
    let height = image.height() as usize;
    let data = image
        .data
        .as_ref()
        .ok_or(ClipboardError::ConversionFailure)?;
    if data.len()
        != width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ClipboardError::ConversionFailure)?
    {
        return Err(ClipboardError::ConversionFailure);
    }

    Ok(arboard::ImageData {
        width,
        height,
        bytes: Cow::Borrowed(data.as_slice()),
    })
}

/// A resource which provides access to the system clipboard.
///
/// Use [`Clipboard::fetch_text`] to read text from the clipboard,
/// and [`Clipboard::set_text`] to write text to the clipboard.
///
/// ## Warning: `system_clipboard` support is off-by-default
///
/// When the `system_clipboard` feature is disabled, operations read from and write to
/// an in-process [`String`] buffer rather than the clipboard provided by the operating system.
/// This means that you will not be able to copy and paste between your application and other applications,
/// and clipboard contents will not persist after your application exits.
/// This is a secure-by-default setup, but is not correct for many applications which require clipboard functionality.
///
/// The fallback is intended to allow clipboard functionality on platforms where `arboard` is not available (e.g. Android, iOS),
/// and to allow applications to have basic clipboard-like functionality without requiring enhanced permissions.
///
/// ## Warning: multithreading deadlock risks
///
/// As the [`arboard`] documentation [warns](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html#windows),
/// accessing the system clipboard on Windows can cause deadlocks if multiple threads or processes attempt to access it simultaneously.
/// Typical usage of the [`Clipboard`] resource should not encounter this issue: Bevy's copy of the [`Clipboard`] resource is unique,
/// and both reading from and writing to it requires exclusive access, enforced by Rust's borrowing rules.
///
/// However, care should be taken to avoid cloning the [`Clipboard`] resource, duplicating it between worlds, reading from it in parallel,
/// or otherwise sharing it across threads, as this could lead to multiple instances attempting to access the clipboard simultaneously and causing a deadlock.
#[derive(Resource)]
pub struct Clipboard {
    #[cfg(all(any(unix, windows), feature = "system_clipboard"))]
    system_clipboard: Option<arboard::Clipboard>,
    // Unfortunately, this cannot be simplified to `not(any(feature = "system_clipboard", target_arch = "wasm32"))`.
    // `system_clipboard` is a platform-conditional dependency (windows/unix only), so on other platforms
    // (Android, iOS, etc.) `cfg(feature = "system_clipboard")` can be true even though the crate is not
    // present. Removing the platform guard would leave those targets with an empty struct and a
    // broken fallback. wasm32 is excluded separately because it calls web-sys directly and stores
    // no state in the struct.
    #[cfg(not(any(
        all(any(windows, unix), feature = "system_clipboard"),
        target_arch = "wasm32"
    )))]
    text: String,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            #[cfg(all(any(unix, windows), feature = "system_clipboard"))]
            system_clipboard: arboard::Clipboard::new().ok(),
            #[cfg(not(any(
                all(any(windows, unix), feature = "system_clipboard"),
                target_arch = "wasm32"
            )))]
            text: String::new(),
        }
    }
}

impl Clipboard {
    /// Fetches UTF-8 text from the clipboard and returns it via a `ClipboardRead`.
    ///
    /// On Windows and Unix `ClipboardRead`s are completed instantly, on wasm32 the result is fetched asynchronously.
    pub fn fetch_text(&mut self) -> ClipboardRead {
        #[cfg(all(any(unix, windows), feature = "system_clipboard"))]
        {
            ClipboardRead::Ready(
                self.system_clipboard
                    .as_mut()
                    .ok_or(ClipboardError::ClipboardNotSupported)
                    .and_then(|clipboard| clipboard.get_text().map_err(ClipboardError::from)),
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
                    if let Ok(mut guard) = shared.lock() {
                        guard.replace(text);
                    }
                });
                ClipboardRead::Pending(shared_clone)
            } else {
                ClipboardRead::Ready(Err(ClipboardError::ClipboardNotSupported))
            }
        }

        #[cfg(not(any(
            all(any(windows, unix), feature = "system_clipboard"),
            target_arch = "wasm32"
        )))]
        {
            #[cfg(any(windows, unix))]
            bevy_log::warn_once!(
                "Clipboard read used an in-process fallback buffer rather than the OS clipboard. \
                 Enable the `system_clipboard` feature on `bevy_clipboard` to use the OS clipboard."
            );
            ClipboardRead::Ready(Ok(self.text.clone()))
        }
    }

    /// Fetches image data from the clipboard.
    ///
    /// Only supported on Windows and Unix platforms with the `image` feature enabled.
    #[cfg(feature = "image")]
    pub fn fetch_image(&mut self) -> Result<Image, ClipboardError> {
        self.system_clipboard
            .as_mut()
            .ok_or(ClipboardError::ClipboardNotSupported)
            .and_then(|clipboard| {
                clipboard
                    .get_image()
                    .map_err(ClipboardError::from)
                    .and_then(try_image_from_imagedata)
            })
    }

    /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
    ///
    /// # Errors
    ///
    /// Returns error if `text` failed to be stored on the clipboard.
    pub fn set_text<'a, T: Into<Cow<'a, str>>>(&mut self, text: T) -> Result<(), ClipboardError> {
        #[cfg(all(any(unix, windows), feature = "system_clipboard"))]
        {
            self.system_clipboard
                .as_mut()
                .ok_or(ClipboardError::ClipboardNotSupported)
                .and_then(|clipboard| clipboard.set_text(text).map_err(ClipboardError::from))
        }

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .map(|w| w.navigator().clipboard())
                .ok_or(ClipboardError::ClipboardNotSupported)
                .map(|clipboard| {
                    let text = text.into().to_string();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(e) = JsFuture::from(clipboard.write_text(&text)).await {
                            bevy_log::warn!("Failed to write text to clipboard: {e:?}");
                        }
                    });
                })
        }

        #[cfg(not(any(
            all(any(windows, unix), feature = "system_clipboard"),
            target_arch = "wasm32"
        )))]
        {
            #[cfg(any(windows, unix))]
            bevy_log::warn_once!(
                "Clipboard write used an in-process fallback buffer rather than the OS clipboard. \
                 Enable the `system_clipboard` feature on `bevy_clipboard` to use the OS clipboard."
            );
            self.text = text.into().into_owned();
            Ok(())
        }
    }

    /// Places image data onto the clipboard.
    ///
    /// The image must contain initialized 2D pixel data in packed RGBA8 row-major order.
    /// Only supported on Windows and Unix platforms with the `image` feature enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the image data is invalid or the clipboard write fails.
    #[cfg(feature = "image")]
    pub fn set_image(&mut self, image: &Image) -> Result<(), ClipboardError> {
        self.system_clipboard
            .as_mut()
            .ok_or(ClipboardError::ClipboardNotSupported)
            .and_then(|clipboard| {
                clipboard
                    .set_image(try_imagedata_from_image(image)?)
                    .map_err(ClipboardError::from)
            })
    }
}

/// An error that might happen during a clipboard operation.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ClipboardError {
    /// Clipboard contents were unavailable or not in the expected format.
    ContentNotAvailable,

    /// No suitable clipboard backend was available
    ClipboardNotSupported,

    /// Clipboard access is temporarily locked by another process or thread.
    ClipboardOccupied,

    /// The data could not be converted to or from the required format.
    ConversionFailure,

    /// An unknown error
    Unknown {
        /// String describing the error
        description: String,
    },
}

impl core::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ContentNotAvailable => {
                write!(
                    f,
                    "clipboard contents were unavailable or not in the expected format"
                )
            }
            Self::ClipboardNotSupported => {
                write!(f, "no suitable clipboard backend was available")
            }
            Self::ClipboardOccupied => {
                write!(
                    f,
                    "clipboard access is temporarily locked by another process or thread"
                )
            }
            Self::ConversionFailure => {
                write!(
                    f,
                    "data could not be converted to or from the required format"
                )
            }
            Self::Unknown { description } => write!(f, "unknown clipboard error: {description}"),
        }
    }
}

impl core::error::Error for ClipboardError {}

#[cfg(all(any(windows, unix), feature = "system_clipboard"))]
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
