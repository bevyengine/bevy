#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;

pub mod prelude {
    pub use crate::{
        dynamic_texture_atlas_builder::DynamicTextureAtlasBuilder,
        texture_atlas::{TextureAtlas, TextureAtlasLayout, TextureAtlasSources},
        BevyDefault as _, Image, ImageFormat, ImagePlugin, TextureAtlasBuilder, TextureError,
    };
}

#[cfg(all(feature = "zstd", not(feature = "zstd_rust"), not(feature = "zstd_c")))]
compile_error!(
    "Choosing a zstd backend is required for zstd support. Please enable either the \"zstd_rust\" or the \"zstd_c\" feature."
);

mod image;
pub use self::image::*;
#[cfg(feature = "serialize")]
mod serialized_image;
#[cfg(feature = "serialize")]
pub use self::serialized_image::*;
#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "compressed_image_saver")]
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

#[cfg(feature = "compressed_image_saver")]
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
