//! Integration test: `SubpixelBucket` partitions the glyph cache so that
//! four distinct horizontal subpixel offsets of the same glyph produce four
//! distinct [`GlyphCacheKey`] entries within a single [`FontAtlas`].
//!
//! Non-subpixel smoothing modes collapse to [`SubpixelBucket::NotApplicable`],
//! giving a single atlas cell per glyph id regardless of fractional position.
//!
//! This test drives [`add_glyph_to_atlas`] directly — the same entry point
//! `TextPipeline::update_text_layout_info` uses once per glyph run — and
//! mirrors the `subpixel_rasterisation.rs` test's scaffolding: a swash
//! `FontRef` + `ScaleContext` rather than a full `App`.

use bevy_asset::Assets;
use bevy_image::Image;
use bevy_math::Vec2;
use bevy_text::{add_glyph_to_atlas, FontAtlas, FontSmoothing, GlyphCacheKey, SubpixelBucket};
use swash::{scale::ScaleContext, FontRef};

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/FiraMono-Medium.ttf");

const FRACTIONAL_XS: [f32; 4] = [0.1, 0.3, 0.6, 0.8];

fn expected_buckets() -> [SubpixelBucket; 4] {
    [
        SubpixelBucket::Zero,
        SubpixelBucket::Quarter,
        SubpixelBucket::Half,
        SubpixelBucket::ThreeQuarter,
    ]
}

/// Return a fresh `Assets<Image>` and the glyph id for `'g'` in the bundled
/// `FiraMono` font — the two pieces needed to drive [`add_glyph_to_atlas`]
/// without pulling in a full `App`.
fn make_image_assets() -> (Assets<Image>, u16) {
    let font = FontRef::from_index(FONT_BYTES, 0).expect("load `FiraMono` font");
    let glyph_id = font.charmap().map('g');
    assert!(glyph_id != 0, "font is missing a glyph for 'g'");

    (Assets::<Image>::default(), glyph_id)
}

fn rasterise_bucket(
    font_atlases: &mut Vec<FontAtlas>,
    textures: &mut Assets<Image>,
    font_size: f32,
    glyph_id: u16,
    font_smoothing: FontSmoothing,
    fractional_x: f32,
) {
    let font = FontRef::from_index(FONT_BYTES, 0).expect("load `FiraMono` font");
    let mut scale_cx = ScaleContext::new();
    let mut scaler = scale_cx.builder(font).size(font_size).hint(true).build();

    let subpixel_bucket = SubpixelBucket::from_fract(fractional_x, font_smoothing);
    let subpixel_offset = Vec2::new(subpixel_bucket.rasterise_offset_x(), 0.0);

    add_glyph_to_atlas(
        font_atlases,
        textures,
        &mut scaler,
        font_smoothing,
        glyph_id,
        subpixel_bucket,
        subpixel_offset,
    )
    .expect("add_glyph_to_atlas failed");
}

#[test]
fn four_distinct_subpixel_buckets_produce_four_cache_entries_in_one_atlas() {
    let (mut textures, glyph_id) = make_image_assets();
    let mut font_atlases: Vec<FontAtlas> = Vec::new();

    for fractional_x in FRACTIONAL_XS {
        rasterise_bucket(
            &mut font_atlases,
            &mut textures,
            24.0,
            glyph_id,
            FontSmoothing::SubpixelAntiAliased,
            fractional_x,
        );
    }

    assert_eq!(
        font_atlases.len(),
        1,
        "four subpixel-bucketed rasterisations of the same glyph should share \
         a single atlas texture — the bucket lives on `GlyphCacheKey` (inner \
         cache key), not `FontAtlasKey` (outer atlas selector); finding more \
         than one atlas here means the bucket leaked to the outer key",
    );

    let atlas = &font_atlases[0];
    assert_eq!(
        atlas.glyph_to_atlas_index.len(),
        4,
        "expected four distinct `GlyphCacheKey` entries (one per bucket); got {}",
        atlas.glyph_to_atlas_index.len(),
    );

    for bucket in expected_buckets() {
        let key = GlyphCacheKey {
            glyph_id,
            subpixel_bucket: bucket,
        };
        assert!(
            atlas.has_glyph(key),
            "atlas is missing an entry for {bucket:?}; \
             `add_glyph_to_atlas` must populate `GlyphCacheKey {{ glyph_id, \
             subpixel_bucket }}` for each rasterised bucket",
        );
    }
}

#[test]
fn repopulating_same_buckets_is_a_pure_cache_hit() {
    let (mut textures, glyph_id) = make_image_assets();
    let mut font_atlases: Vec<FontAtlas> = Vec::new();

    for fractional_x in FRACTIONAL_XS {
        rasterise_bucket(
            &mut font_atlases,
            &mut textures,
            24.0,
            glyph_id,
            FontSmoothing::SubpixelAntiAliased,
            fractional_x,
        );
    }

    let entries_after_first_pass = font_atlases[0].glyph_to_atlas_index.len();
    assert_eq!(entries_after_first_pass, 4, "setup invariant");

    // Re-request the same four buckets. `add_glyph_to_atlas` must notice the
    // entries exist and return without inserting new ones — a `HashMap::insert`
    // for an already-present key is a silent overwrite, but the entry count
    // stays put.
    for fractional_x in FRACTIONAL_XS {
        rasterise_bucket(
            &mut font_atlases,
            &mut textures,
            24.0,
            glyph_id,
            FontSmoothing::SubpixelAntiAliased,
            fractional_x,
        );
    }

    assert_eq!(
        font_atlases.len(),
        1,
        "cache-hit re-population spawned a new atlas",
    );
    assert_eq!(
        font_atlases[0].glyph_to_atlas_index.len(),
        entries_after_first_pass,
        "cache-hit re-population should not add new entries",
    );
}

#[test]
fn non_subpixel_smoothing_collapses_to_single_not_applicable_entry() {
    let (mut textures, glyph_id) = make_image_assets();
    let mut font_atlases: Vec<FontAtlas> = Vec::new();

    for fractional_x in FRACTIONAL_XS {
        rasterise_bucket(
            &mut font_atlases,
            &mut textures,
            24.0,
            glyph_id,
            FontSmoothing::AntiAliased,
            fractional_x,
        );
    }

    assert_eq!(
        font_atlases.len(),
        1,
        "`AntiAliased` rasterisations of a single glyph should share one atlas",
    );

    let atlas = &font_atlases[0];
    assert_eq!(
        atlas.glyph_to_atlas_index.len(),
        1,
        "`AntiAliased` always maps to `SubpixelBucket::NotApplicable`, so four \
         fractional-x rasterisations of the same glyph must collapse to one \
         cache entry; got {}",
        atlas.glyph_to_atlas_index.len(),
    );

    let key = GlyphCacheKey {
        glyph_id,
        subpixel_bucket: SubpixelBucket::NotApplicable,
    };
    assert!(
        atlas.has_glyph(key),
        "expected cache entry with `SubpixelBucket::NotApplicable`",
    );
}
