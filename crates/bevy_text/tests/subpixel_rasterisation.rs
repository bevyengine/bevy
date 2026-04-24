//! Integration test: `FontSmoothing::SubpixelAntiAliased` produces
//! RGB-distinguishable glyph output.
//!
//! The grayscale (`AntiAliased`) path writes `[255, 255, 255, a]` per pixel,
//! so every pixel satisfies `R == G == B`. The subpixel path writes per-
//! channel coverage (`[R_cov, G_cov, B_cov, _]`), so at least one pixel of
//! any real glyph raster should have `R != G` or `G != B`.
//!
//! This test exercises `get_outlined_glyph_texture` directly via a swash
//! `FontRef` + `ScaleContext`, which mirrors the setup
//! `TextPipeline::update_text_layout_info` performs per glyph run.

use bevy_image::Image;
use bevy_math::Vec2;
use bevy_text::{get_outlined_glyph_texture, FontSmoothing, TextError};
use swash::{scale::ScaleContext, FontRef};

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/FiraMono-Medium.ttf");

/// Rasterise a single character from the bundled `FiraMono` font under the
/// requested `font_smoothing`, returning the full result of
/// [`get_outlined_glyph_texture`].
fn rasterise(
    scale_cx: &mut ScaleContext,
    ch: char,
    font_size: f32,
    font_smoothing: FontSmoothing,
) -> Result<(Image, Vec2, bool), TextError> {
    let font = FontRef::from_index(FONT_BYTES, 0).expect("load `FiraMono` font");
    let glyph_id = font.charmap().map(ch);
    assert!(glyph_id != 0, "font is missing a glyph for {ch:?}");
    let mut scaler = scale_cx.builder(font).size(font_size).hint(true).build();
    get_outlined_glyph_texture(&mut scaler, glyph_id, font_smoothing)
}

#[test]
fn subpixel_rasterisation_produces_non_grayscale_output() {
    let mut scale_cx = ScaleContext::new();

    // Grayscale baseline: every pixel must have R == G == B.
    let (grayscale_image, _, grayscale_is_alpha_mask) =
        rasterise(&mut scale_cx, 'g', 24.0, FontSmoothing::AntiAliased)
            .expect("grayscale rasterisation failed");
    let grayscale_data = grayscale_image
        .data
        .as_ref()
        .expect("grayscale image missing CPU data");

    assert!(
        grayscale_is_alpha_mask,
        "grayscale AntiAliased output should report is_alpha_mask=true",
    );
    assert!(
        grayscale_data
            .chunks_exact(4)
            .all(|px| px[0] == px[1] && px[1] == px[2]),
        "grayscale AntiAliased output unexpectedly had differing R/G/B channels",
    );

    // Subpixel output: at least one pixel must have R != G or G != B.
    let (subpixel_image, _, subpixel_is_alpha_mask) =
        rasterise(&mut scale_cx, 'g', 24.0, FontSmoothing::SubpixelAntiAliased)
            .expect("subpixel rasterisation failed");
    let subpixel_data = subpixel_image
        .data
        .as_ref()
        .expect("subpixel image missing CPU data");

    assert!(
        !subpixel_is_alpha_mask,
        "subpixel output should report is_alpha_mask=false so the UI \
         pipeline knows to composite it with a dual-source blend",
    );
    assert!(
        subpixel_image.width() > 0 && subpixel_image.height() > 0,
        "subpixel rasterisation produced a zero-sized image",
    );

    let any_rgb_differs = subpixel_data
        .chunks_exact(4)
        .any(|px| px[0] != px[1] || px[1] != px[2]);
    assert!(
        any_rgb_differs,
        "subpixel SubpixelAntiAliased output had R == G == B in every pixel, \
         indicating Format::Subpixel was not used for rasterisation",
    );
}
