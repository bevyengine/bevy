// FIXME(15321): solve CI failures, then replace with `#![expect()]`.
#![allow(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![allow(unsafe_code)]

mod image;
pub use self::image::*;
#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "dds")]
mod dds;
#[cfg(feature = "exr")]
mod exr_texture_loader;
#[cfg(feature = "hdr")]
mod hdr_texture_loader;
#[cfg(feature = "ktx2")]
mod ktx2;

#[cfg(feature = "ktx2")]
pub use self::ktx2::*;
#[cfg(feature = "dds")]
pub use dds::*;
#[cfg(feature = "exr")]
pub use exr_texture_loader::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;

pub(crate) mod image_texture_conversion;
pub use image_texture_conversion::IntoDynamicImageError;
