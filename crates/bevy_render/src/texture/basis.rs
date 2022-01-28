use basis_universal::{
    BasisTextureType, DecodeFlags, TranscodeParameters, Transcoder, TranscoderTextureFormat,
};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use super::{CompressedImageFormats, Image, TextureError};

pub fn basis_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> Result<Image, TextureError> {
    let mut transcoder = Transcoder::new();

    #[cfg(debug_assertions)]
    if !transcoder.validate_file_checksums(buffer, true) {
        return Err(TextureError::InvalidData("Invalid checksum".to_string()));
    }
    if !transcoder.validate_header(buffer) {
        return Err(TextureError::InvalidData("Invalid header".to_string()));
    }

    assert_eq!(transcoder.image_count(buffer), 1);
    let image_index = 0;

    let image_info = if let Some(image_info) = transcoder.image_info(buffer, image_index) {
        image_info
    } else {
        return Err(TextureError::InvalidData(
            "Failed to get image info".to_string(),
        ));
    };

    // First deal with transcoding to the desired format
    // FIXME: Use external metadata to transcode to more appropriate formats for 1- or 2-component sources
    let (transcode_format, texture_format) =
        get_transcoded_formats(supported_compressed_formats, is_srgb);
    let basis_texture_format = transcoder.basis_texture_format(buffer);
    if !basis_texture_format.can_transcode_to_format(transcode_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "{:?} cannot be transcoded to {:?}",
            basis_texture_format, transcode_format
        )));
    }
    transcoder.prepare_transcoding(buffer).map_err(|_| {
        TextureError::TranscodeError(format!(
            "Failed to prepare for transcoding from {:?}",
            basis_texture_format
        ))
    })?;
    let mut transcoded = Vec::new();
    let mip_level_count = transcoder.image_level_count(buffer, image_index);
    for level_index in 0..mip_level_count {
        let mut data = transcoder
            .transcode_image_level(
                buffer,
                transcode_format,
                TranscodeParameters {
                    image_index,
                    level_index,
                    decode_flags: Some(DecodeFlags::HIGH_QUALITY),
                    ..Default::default()
                },
            )
            .map_err(|error| {
                TextureError::TranscodeError(format!(
                    "Failed to transcode mip level {} from {:?} to {:?}: {:?}",
                    level_index, basis_texture_format, transcode_format, error
                ))
            })?;
        transcoded.append(&mut data);
    }

    // Then prepare the Image
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: image_info.m_orig_width,
        height: image_info.m_orig_height,
        // FIXME: Support 3D and array textures
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.mip_level_count = mip_level_count;
    image.texture_descriptor.format = texture_format;
    image.texture_descriptor.dimension = match transcoder.basis_texture_type(buffer) {
        BasisTextureType::TextureType2D => TextureDimension::D2,
        basis_texture_type => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                basis_texture_type
            )));
        }
    };
    image.data = transcoded;
    Ok(image)
}

pub fn get_transcoded_formats(
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> (TranscoderTextureFormat, TextureFormat) {
    if supported_compressed_formats.contains(CompressedImageFormats::BC) {
        (
            TranscoderTextureFormat::BC7_RGBA,
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            },
        )
    } else if supported_compressed_formats.contains(CompressedImageFormats::ASTC_LDR) {
        (
            TranscoderTextureFormat::ASTC_4x4_RGBA,
            if is_srgb {
                TextureFormat::Astc4x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc4x4RgbaUnorm
            },
        )
    } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
        (
            TranscoderTextureFormat::ETC2_RGBA,
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            },
        )
    } else {
        (
            TranscoderTextureFormat::RGBA32,
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
        )
    }
}
