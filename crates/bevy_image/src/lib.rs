#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;

pub mod prelude {
    pub use crate::{
        dynamic_texture_atlas_builder::DynamicTextureAtlasBuilder,
        texture_atlas::{TextureAtlas, TextureAtlasLayout, TextureAtlasSources},
        BevyDefault as _, Image, ImageFormat, TextureAtlasBuilder, TextureError,
    };
}

mod image;
pub use self::image::*;
#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "basis-universal")]
mod compressed_image_saver;
#[cfg(feature = "dds")]
mod dds;
mod dynamic_texture_atlas_builder;
#[cfg(feature = "exr")]
mod exr_texture_loader;
#[cfg(feature = "hdr")]
mod hdr_texture_loader;
mod image_loader;
#[cfg(feature = "ktx2")]
mod ktx2;
mod texture_atlas;
mod texture_atlas_builder;

#[cfg(feature = "basis-universal")]
pub use compressed_image_saver::*;
#[cfg(feature = "dds")]
pub use dds::*;
pub use dynamic_texture_atlas_builder::*;
#[cfg(feature = "exr")]
pub use exr_texture_loader::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;
pub use image_loader::*;
#[cfg(feature = "ktx2")]
pub use ktx2::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

pub(crate) mod image_texture_conversion;
pub use image_texture_conversion::IntoDynamicImageError;
