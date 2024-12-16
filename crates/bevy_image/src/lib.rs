#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![allow(unsafe_code)]

pub mod prelude {
    pub use crate::{BevyDefault as _, Image, ImageFormat, TextureError};
}

mod image;
pub use self::image::*;
#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "basis-universal")]
mod compressed_image_saver;
#[cfg(feature = "dds")]
mod dds;
#[cfg(feature = "exr")]
mod exr_texture_loader;
#[cfg(feature = "hdr")]
mod hdr_texture_loader;
mod image_loader;
#[cfg(feature = "ktx2")]
mod ktx2;

#[cfg(feature = "basis-universal")]
pub use compressed_image_saver::*;
#[cfg(feature = "dds")]
pub use dds::*;
#[cfg(feature = "exr")]
pub use exr_texture_loader::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;
pub use image_loader::*;
#[cfg(feature = "ktx2")]
pub use ktx2::*;

pub(crate) mod image_texture_conversion;
pub use image_texture_conversion::IntoDynamicImageError;
