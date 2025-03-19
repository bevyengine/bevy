use crate::{CompressedImageFormats, Image, TextureError};

/// Decodes/transcodes a KTX2 image.
///
/// If you have an image with special needs that Bevy's rust frontend doesn't handle well (or
/// you know will always be kicked out to `basis-universal-sys`, like a ETC1S/BasisLZ image),
/// feel free to use [`crate::ktx2_buffer_to_image_using_basisu`] directly.
pub fn ktx2_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
) -> Result<Image, TextureError> {
    return crate::ktx2_using_rust::ktx2_buffer_to_image_using_rust(
        buffer,
        supported_compressed_formats,
    );
}
