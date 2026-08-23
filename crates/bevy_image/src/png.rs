use crate::SourceColorPrimaries;
use {bevy_utils::once, tracing::warn};

/// Reads the PNG `cICP` chunk and matches it against the supported
/// [`SourceColorPrimaries`].
///
/// Returns `None` when the chunk is absent, the header cannot be parsed, or the
/// primaries are not supported. Unsupported primaries warn once, and so does a PQ or
/// HLG transfer function, since the data is loaded as if it were sRGB-encoded.
pub(crate) fn png_source_color_primaries(bytes: &[u8]) -> Option<SourceColorPrimaries> {
    // Errors are swallowed. This is best-effort metadata, and any structural problem
    // with the file surfaces in the decode instead. `read_info` stops at the image
    // data, and the ignore flags keep it from inflating the ICC profile, buffering
    // text chunks, or checksumming on the way there.
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_ignore_text_chunk(true);
    decoder.ignore_checksums(true);
    let reader = decoder.read_info().ok()?;
    let cicp = reader.info().coding_independent_code_points?;
    // The codes come from ITU-T H.273. 16 is PQ and 18 is HLG.
    if let transfer @ (16 | 18) = cicp.transfer_function {
        let name = if transfer == 16 { "PQ" } else { "HLG" };
        once!(warn!(
            "PNG file declares the {name} transfer function, which Bevy does not support. \
            The data is loaded unchanged. Re-encode the file with an sRGB transfer \
            function to display it correctly.",
        ));
    }
    let source_color_primaries = cicp_to_source_color_primaries(cicp.color_primaries);
    if source_color_primaries.is_none() {
        once!(warn!(
            "PNG file declares cICP color primaries code {}, which Bevy does not support. \
            Assuming BT.709 primaries.",
            cicp.color_primaries,
        ));
    }
    source_color_primaries
}

/// Maps a cICP color primaries code from ITU-T H.273 to [`SourceColorPrimaries`].
/// Returns `None` for unsupported primaries.
fn cicp_to_source_color_primaries(code: u8) -> Option<SourceColorPrimaries> {
    match code {
        1 => Some(SourceColorPrimaries::Bt709),
        9 => Some(SourceColorPrimaries::Bt2020),
        12 => Some(SourceColorPrimaries::DisplayP3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a 1x1 grayscale PNG to memory, with a `cICP` chunk in front of the image
    /// data when given. The chunk goes through `write_chunk`, since the `png` crate
    /// decodes `cICP` but does not encode it.
    fn write_test_png(cicp: Option<[u8; 2]>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        if let Some([color_primaries, transfer_function]) = cicp {
            writer
                .write_chunk(
                    png::chunk::ChunkType(*b"cICP"),
                    &[
                        color_primaries,
                        transfer_function,
                        /* matrix: RGB */ 0,
                        /* full range */ 1,
                    ],
                )
                .unwrap();
        }
        writer.write_image_data(&[128]).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn cicp_primaries_are_read_from_the_chunk() {
        for (code, expected) in [
            (1, SourceColorPrimaries::Bt709),
            (9, SourceColorPrimaries::Bt2020),
            (12, SourceColorPrimaries::DisplayP3),
        ] {
            assert_eq!(
                png_source_color_primaries(&write_test_png(Some([code, /* sRGB */ 13]))),
                Some(expected)
            );
        }
    }

    #[test]
    fn unknown_cicp_primaries_yield_none() {
        // BT.601 (code 6) is a valid file value but not a supported variant.
        assert_eq!(
            png_source_color_primaries(&write_test_png(Some([6, /* sRGB */ 13]))),
            None
        );
    }

    #[test]
    fn png_without_cicp_yields_none() {
        assert_eq!(png_source_color_primaries(&write_test_png(None)), None);
    }

    #[test]
    fn from_buffer_sets_png_cicp_primaries() {
        let image = crate::Image::from_buffer(
            &write_test_png(Some([9, /* PQ */ 16])),
            crate::ImageType::Extension("png"),
            crate::CompressedImageFormats::empty(),
            false,
            crate::ImageSampler::Default,
            bevy_asset::RenderAssetUsages::default(),
            None,
        )
        .unwrap();
        assert_eq!(image.source_color_primaries, SourceColorPrimaries::Bt2020);
    }
}
