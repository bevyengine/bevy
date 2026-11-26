use crate::{Image, ImageSampler};
use alloc::sync::Arc;
use bevy_asset::{io::Reader, Asset, AssetLoader, LoadContext, RenderAssetUsages};
use bevy_math::Vec2;
use bevy_reflect::TypePath;
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::warn;
use wgpu_types::{Extent3d, TextureDataOrder, TextureDimension, TextureFormat};

/// Loads `.svg` files as [`Image`]s.
///
/// Registered by [`ImagePlugin`](crate::ImagePlugin) when the `svg` feature is enabled.
///
/// An SVG is rasterized once at the size chosen by
/// [`SvgLoaderSettings::size`]. The result is an ordinary [`Image`] in
/// [`TextureFormat::Rgba8UnormSrgb`].
///
/// Not supported:
/// - `<image>` elements, whether they point to a file, another SVG or embedded data. They are
///   skipped with a warning, and their targets are never read.
/// - `<text>`, which is skipped with a warning.
/// - Animation, scripting, and `<foreignObject>`.
#[derive(Clone, Debug, Default, TypePath)]
pub struct SvgImageLoader;

/// The largest width or height, in pixels, that an SVG is rasterized at.
///
/// Guards against mistakes such as a huge `viewBox` or scale factor.
const MAX_DIMENSION: u32 = 8192;

/// The pixel size to rasterize an SVG at.
///
/// Sizes are in pixels of the resulting [`Image`]. Except for [`SvgRasterSize::Intrinsic`], the
/// SVG's aspect ratio is kept, and fractional sizes are rounded to the nearest pixel.
///
/// Sizes wider or taller than 8192 pixels are scaled down to fit, keeping their aspect ratio,
/// and a warning is logged.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub enum SvgRasterSize {
    /// The size the SVG declares through its `width` and `height` attributes, or its `viewBox`
    /// when those are missing.
    ///
    /// This is the default.
    #[default]
    Intrinsic,
    /// The intrinsic size multiplied by this factor. Must be greater than zero.
    Scale(f32),
    /// This width, with the height following the aspect ratio.
    Width(u32),
    /// This height, with the width following the aspect ratio.
    Height(u32),
    /// The largest size that fits inside this box.
    Fit {
        /// The width of the box.
        width: u32,
        /// The height of the box.
        height: u32,
    },
}

/// Settings for loading an SVG with [`SvgImageLoader`].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SvgLoaderSettings {
    /// The pixel size of the resulting [`Image`].
    pub size: SvgRasterSize,
    /// Whether to generate a full mip chain from the SVG.
    /// Defaults to `true`.
    /// Mipmaps prevent aliasing when textures are minified and improve GPU cache locality,
    /// so they are almost always wanted for material textures.
    pub generate_mipmap: bool,
    /// The [`ImageSampler`] to use when rendering.
    pub sampler: ImageSampler,
    /// Where the asset will be used. See [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
}

impl Default for SvgLoaderSettings {
    fn default() -> Self {
        Self {
            size: SvgRasterSize::default(),
            generate_mipmap: true,
            sampler: ImageSampler::linear(),
            asset_usage: RenderAssetUsages::default(),
        }
    }
}

/// An error produced while turning SVG data into an [`Image`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SvgError {
    /// The data is not a valid SVG document.
    #[error("failed to parse SVG: {0}")]
    Parse(String),
    /// [`SvgLoaderSettings::size`] asked for an empty or non-finite size.
    #[error("invalid SVG raster size {0:?}: sizes must be finite and greater than zero")]
    InvalidSize(SvgRasterSize),
    /// The pixel buffer for the requested size could not be created.
    #[error("cannot allocate a {width}x{height} pixel buffer for the SVG")]
    Allocation {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// The requested element could not be found.
    #[error("failed to find element with id \"{0}\"")]
    NotFound(String),
}

/// An error produced by [`SvgImageLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SvgLoaderError {
    /// The file could not be read.
    #[error("failed to read SVG file: {0}")]
    Io(#[from] std::io::Error),
    /// The file could not be turned into an image.
    #[error("failed to load SVG file {path}: {error}")]
    Svg {
        /// The asset path of the file.
        path: String,
        /// What went wrong.
        error: SvgError,
    },
}

impl AssetLoader for SvgImageLoader {
    type Asset = Image;
    type Settings = SvgLoaderSettings;
    type Error = SvgLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &SvgLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let path = load_context.path().path();
        rasterize(&bytes, settings, path).map_err(|error| SvgLoaderError::Svg {
            path: path.display().to_string(),
            error,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["svg"]
    }
}

/// A parsed SVG document. Use this if you want to control rasterization yourself.
///
/// Load files with `asset_server.load::<Svg>("icons.svg")`.
#[derive(Asset, TypePath, Clone)]
pub struct Svg {
    tree: Arc<usvg::Tree>,
    path: PathBuf,
}

impl Svg {
    fn parse(bytes: &[u8], path: &Path) -> Result<Self, SvgError> {
        let options = usvg_options(path);
        let tree = usvg::Tree::from_data(bytes, &options)
            .map_err(|error| SvgError::Parse(error.to_string()))?;
        Ok(Self {
            tree: Arc::new(tree),
            path: path.to_path_buf(),
        })
    }

    /// Returns the document's intrinsic size in pixels.
    pub fn size(&self) -> Vec2 {
        let size = self.tree.size();
        Vec2::new(size.width(), size.height())
    }

    /// Check the tree for a node with an id.
    pub fn contains_element_by_id(&self, id: &str) -> bool {
        self.tree.node_by_id(id).is_some()
    }

    /// Rasterize the document to a `Image`
    pub fn rasterize(&self, size: SvgRasterSize) -> Result<Image, SvgError> {
        let document = self.tree.size();
        let (width, height) = raster_size(size, document.width(), document.height(), &self.path)?;
        let transform = tiny_skia::Transform::from_scale(
            width as f32 / document.width(),
            height as f32 / document.height(),
        );
        let mut pixmap =
            tiny_skia::Pixmap::new(width, height).ok_or(SvgError::Allocation { width, height })?;
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());

        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take_demultiplied(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::linear();
        Ok(image)
    }

    /// Rasterize a node in the document to an `Image`
    pub fn rasterize_by_id(&self, id: &str, size: SvgRasterSize) -> Result<Image, SvgError> {
        let not_found = || SvgError::NotFound(id.into());
        let node = self.tree.node_by_id(id).ok_or_else(not_found)?;
        let bounds = node.abs_layer_bounding_box().ok_or_else(not_found)?;
        let (width, height) = raster_size(size, bounds.width(), bounds.height(), &self.path)?;
        let (x, y) = (bounds.x(), bounds.y());
        let parent_transform = parent_transform(node);
        let transform = tiny_skia::Transform::from_scale(
            width as f32 / bounds.width(),
            height as f32 / bounds.height(),
        )
        .pre_translate(-x, -y)
        .pre_concat(parent_transform)
        .pre_translate(x, y);
        let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
        resvg::render_node(node, transform, &mut pixmap.as_mut());

        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take_demultiplied(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::linear();
        Ok(image)
    }
}

/// Asset loader responsible for SVG documents.
#[derive(Clone, Debug, Default, TypePath)]
pub struct SvgDocumentLoader;

impl AssetLoader for SvgDocumentLoader {
    type Asset = Svg;
    type Settings = ();
    type Error = SvgLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Svg, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let path = load_context.path().path();
        Svg::parse(&bytes, path).map_err(|error| SvgLoaderError::Svg {
            path: path.display().to_string(),
            error,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["svg"]
    }
}

/// Parses `bytes` as an SVG and renders it into an [`Image`].
fn rasterize(bytes: &[u8], settings: &SvgLoaderSettings, path: &Path) -> Result<Image, SvgError> {
    let options = usvg_options(path);
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|error| SvgError::Parse(error.to_string()))?;

    if contains_text_element(bytes) {
        warn!(
            "{}: <text> elements in SVGs are not supported",
            path.display()
        );
    }

    let intrinsic = tree.size();
    let (width, height) = raster_size(settings.size, intrinsic.width(), intrinsic.height(), path)?;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let max_mips = size.max_mips(TextureDimension::D2);

    let image: Image = if settings.generate_mipmap {
        // Calculate the size of each mip level
        let sizes: Vec<_> = (0..max_mips)
            .map(|level| size.mip_level_size(level, TextureDimension::D2))
            .collect();

        // Preallocate based on mip levels
        let mut data = Vec::with_capacity(
            sizes
                .iter()
                .map(|s| s.width as usize * s.height as usize * 4)
                .sum(),
        );

        for size in sizes {
            data.extend(render_level(&tree, size.width, size.height)?);
        }

        let mut image = Image::new_uninit(
            size,
            TextureDimension::D2,
            TextureFormat::Rgba8UnormSrgb,
            settings.asset_usage,
        );
        image.data = Some(data);
        image.data_order = TextureDataOrder::MipMajor;
        image.texture_descriptor.mip_level_count = max_mips;
        image.sampler = settings.sampler.clone();
        image
    } else {
        Image::new(
            size,
            TextureDimension::D2,
            render_level(&tree, width, height)?,
            TextureFormat::Rgba8UnormSrgb,
            settings.asset_usage,
        )
    };

    Ok(image)
}

/// Renders the SVG to `width` x `height`.
fn render_level(tree: &usvg::Tree, width: u32, height: u32) -> Result<Vec<u8>, SvgError> {
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or(SvgError::Allocation { width, height })?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / tree.size().width(),
        height as f32 / tree.size().height(),
    );
    resvg::render(tree, transform, &mut pixmap.as_mut());

    // The rasterizer produces premultiplied alpha, but images are expected to be straight alpha.
    let mut data = pixmap.take_demultiplied();
    bleed_edges(&mut data, width as usize, height as usize);
    Ok(data)
}

/// Gives fully transparent pixels the average color of their visible neighbors.
///
/// Transparent pixels rasterize with color channels set to zero,
/// and filters read that as black. Here we use a 3x3 sample to blend
/// the color channels into transparent pixels to prevent that.
fn bleed_edges(data: &mut [u8], width: usize, height: usize) {
    // Works in place: only transparent texels are written, and they carry no weight when read.
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            if data[index + 3] != 0 {
                // Skip the pixel if it isn't fully transparent.
                continue;
            }
            // Sum of the sample's channels
            let mut sum = [0u32; 3];
            // Weight of the sample's alpha
            let mut weight = 0;
            let (prev_row, next_row_end) = (y.saturating_sub(1), (y + 2).min(height));
            for tex_y in prev_row..next_row_end {
                let (prev_column, next_column_end) = (x.saturating_sub(1), (x + 2).min(width));
                for tex_x in prev_column..next_column_end {
                    // Iterate the 3x3 sample
                    let texel = &data[(tex_y * width + tex_x) * 4..][..4];
                    let alpha = u32::from(texel[3]);
                    for (sum, channel) in sum.iter_mut().zip(texel) {
                        *sum += u32::from(*channel) * alpha;
                    }
                    weight += alpha;
                }
            }
            // Skip writing for fully transparent 3x3 samples
            if weight == 0 {
                continue;
            }
            // Write out channel averages
            for (channel, sum) in data[index..index + 3].iter_mut().zip(sum) {
                *channel = (sum / weight) as u8;
            }
        }
    }
}

fn usvg_options(path: &Path) -> usvg::Options<'_> {
    let mut options = usvg::Options::default();

    let skip_image = move |href: &str| {
        let href: String = href.chars().take(64).collect();
        warn!(
            "{}: skipping <image href=\"{href}\">; <image> elements in SVGs are not supported",
            path.display()
        );
        None
    };
    options.image_href_resolver.resolve_string = Box::new(move |href, _| skip_image(href));
    options.image_href_resolver.resolve_data =
        Box::new(move |mime, _, _| skip_image(&format!("data:{mime},...")));
    options
}

/// Determine final raster size from the requested size
fn raster_size(
    requested: SvgRasterSize,
    intrinsic_width: f32,
    intrinsic_height: f32,
    path: &Path,
) -> Result<(u32, u32), SvgError> {
    let invalid = || SvgError::InvalidSize(requested);
    let (width, height) = match requested {
        SvgRasterSize::Intrinsic => (intrinsic_width, intrinsic_height),
        SvgRasterSize::Scale(scale) => {
            if !(scale.is_finite() && scale > 0.0) {
                return Err(invalid());
            }
            (intrinsic_width * scale, intrinsic_height * scale)
        }
        SvgRasterSize::Width(width) => {
            if width == 0 {
                return Err(invalid());
            }
            let width = width as f32;
            (width, intrinsic_height * width / intrinsic_width)
        }
        SvgRasterSize::Height(height) => {
            if height == 0 {
                return Err(invalid());
            }
            let height = height as f32;
            (intrinsic_width * height / intrinsic_height, height)
        }
        SvgRasterSize::Fit { width, height } => {
            if width == 0 || height == 0 {
                return Err(invalid());
            }
            let scale = (width as f32 / intrinsic_width).min(height as f32 / intrinsic_height);
            (intrinsic_width * scale, intrinsic_height * scale)
        }
    };

    let max_dimension = MAX_DIMENSION as f32;
    let largest = width.max(height);
    let (width, height) = if largest > max_dimension {
        let scale = max_dimension / largest;
        warn!(
            "{}: SVG raster size {width:.0}x{height:.0} exceeds the maximum of {MAX_DIMENSION} \
            pixels and was scaled down",
            path.display()
        );
        (width * scale, height * scale)
    } else {
        (width, height)
    };

    let to_pixels = |value: f32| (value.round() as u32).clamp(1, MAX_DIMENSION);
    Ok((to_pixels(width), to_pixels(height)))
}

/// Check the SVG source for `<text>` elements
fn contains_text_element(bytes: &[u8]) -> bool {
    bytes.windows(5).any(|window| window == b"<text")
}

fn parent_transform(node: &usvg::Node) -> tiny_skia::Transform {
    match node {
        usvg::Node::Group(group) => group
            .transform()
            .invert()
            .map(|own| group.abs_transform().pre_concat(own))
            .unwrap_or_default(),
        _ => node.abs_transform(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_color::{Color, ColorToComponents};

    fn load(svg: &str, settings: &SvgLoaderSettings) -> Result<Image, SvgError> {
        rasterize(svg.as_bytes(), settings, Path::new("test.svg"))
    }

    fn load_default(svg: &str) -> Image {
        load(svg, &SvgLoaderSettings::default()).unwrap()
    }

    fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
        let bytes = image.pixel_bytes((x, y, 0).into()).unwrap();
        bytes.try_into().unwrap()
    }

    /// A 10x20 SVG with the top half red and the bottom half transparent.
    const HALF_RED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20">
        <rect width="10" height="10" fill="red"/>
    </svg>"#;

    #[test]
    fn intrinsic_size_and_pixels() {
        let image = load_default(HALF_RED);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 20);
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(rgba(&image, 5, 2), [255, 0, 0, 255]);
        assert_eq!(rgba(&image, 5, 17), [0, 0, 0, 0]);
    }

    #[test]
    fn full_mip_chain_by_default() {
        let image = load_default(HALF_RED);
        assert_eq!(image.data_order, TextureDataOrder::MipMajor);
        // 10x20, 5x10, 2x5, 1x2, 1x1
        assert_eq!(image.texture_descriptor.mip_level_count, 5);
        assert_eq!(
            image.data.as_ref().unwrap().len(),
            (200 + 50 + 10 + 2 + 1) * 4
        );
    }

    #[test]
    fn mip_level_count_follows_settings() {
        for (mipmap, expected) in [(false, 1), (true, 5)] {
            let settings = SvgLoaderSettings {
                generate_mipmap: mipmap,
                ..Default::default()
            };
            let image = load(HALF_RED, &settings).unwrap();
            assert_eq!(
                image.texture_descriptor.mip_level_count, expected,
                "{mipmap:?}"
            );
        }
        let settings = SvgLoaderSettings {
            generate_mipmap: false,
            ..Default::default()
        };
        assert_eq!(
            load(HALF_RED, &settings).unwrap().data.unwrap().len(),
            10 * 20 * 4
        );
    }

    #[test]
    fn mip_levels_hold_the_whole_image() {
        // 8x8 image with a red 4x8 rectangle on the left side.
        // Mip level 2 starts after level 0 (offset 64) and level 1 (offset 16)
        // Its top row should have a opaque red pixel and a transparent pixel with red bled into it.
        let image = load_default(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8">
                <rect width="4" height="8" fill="red"/>
            </svg>"#,
        );
        let level_2 = &image.data.as_ref().unwrap()[(64 + 16) * 4..][..2 * 2 * 4];
        assert_eq!(&level_2[..4], &[255, 0, 0, 255]);
        assert_eq!(&level_2[4..8], &[255, 0, 0, 0]);
    }

    #[test]
    fn transparent_edges_take_neighbor_color() {
        let image = load_default(HALF_RED);
        assert_eq!(rgba(&image, 5, 10), [255, 0, 0, 0]);
        assert_eq!(rgba(&image, 5, 11), [0, 0, 0, 0]);
    }

    #[test]
    fn view_box_only_uses_view_box_size() {
        let image =
            load_default(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 12"></svg>"#);
        assert_eq!((image.width(), image.height()), (24, 12));
    }

    #[test]
    fn requested_sizes_keep_aspect_ratio() {
        let cases = [
            (SvgRasterSize::Scale(2.0), (20, 40)),
            (SvgRasterSize::Width(5), (5, 10)),
            (SvgRasterSize::Height(40), (20, 40)),
            (
                SvgRasterSize::Fit {
                    width: 100,
                    height: 30,
                },
                (15, 30),
            ),
        ];
        for (size, expected) in cases {
            let settings = SvgLoaderSettings {
                size,
                ..Default::default()
            };
            let image = load(HALF_RED, &settings).unwrap();
            assert_eq!((image.width(), image.height()), expected, "{size:?}");
        }
    }

    #[test]
    fn scaled_content_fills_the_image() {
        let settings = SvgLoaderSettings {
            size: SvgRasterSize::Scale(3.0),
            ..Default::default()
        };
        let image = load(HALF_RED, &settings).unwrap();
        assert_eq!(rgba(&image, 29, 28), [255, 0, 0, 255]);
        assert_eq!(rgba(&image, 29, 31), [0, 0, 0, 0]);
    }

    #[test]
    fn invalid_sizes_are_rejected() {
        for size in [
            SvgRasterSize::Scale(0.0),
            SvgRasterSize::Scale(-1.0),
            SvgRasterSize::Scale(f32::NAN),
            SvgRasterSize::Scale(f32::INFINITY),
            SvgRasterSize::Width(0),
            SvgRasterSize::Height(0),
            SvgRasterSize::Fit {
                width: 0,
                height: 10,
            },
        ] {
            let settings = SvgLoaderSettings {
                size,
                ..Default::default()
            };
            assert!(
                matches!(load(HALF_RED, &settings), Err(SvgError::InvalidSize(_))),
                "{size:?}"
            );
        }
    }

    #[test]
    fn oversized_requests_are_clamped() {
        let path = Path::new("test.svg");
        let size = raster_size(SvgRasterSize::Height(100_000), 10.0, 20.0, path).unwrap();
        assert_eq!(size, (4096, 8192));
        let size = raster_size(SvgRasterSize::Intrinsic, 1_000_000.0, 1.0, path).unwrap();
        assert_eq!(size, (8192, 1));
    }

    #[test]
    fn malformed_input_is_an_error() {
        let settings = SvgLoaderSettings::default();
        assert!(matches!(
            load("not svg", &settings),
            Err(SvgError::Parse(_))
        ));
        assert!(matches!(
            load(
                r#"<svg xmlns="http://www.w3.org/2000/svg"><rect"#,
                &settings
            ),
            Err(SvgError::Parse(_))
        ));
    }

    #[test]
    fn edges_are_straight_alpha() {
        // A half-covered pixel of an opaque white shape must stay white with partial alpha.
        // With premultiplied data it would come out grey.
        let image = load_default(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4">
                <rect width="2.5" height="4" fill="white"/>
            </svg>"#,
        );
        let [r, g, b, a] = rgba(&image, 2, 1);
        assert!(
            a > 64 && a < 192,
            "expected partial coverage, got alpha {a}"
        );
        assert!(r >= 254 && g >= 254 && b >= 254, "got {r} {g} {b}");
    }

    #[test]
    fn drawn_as_authored() {
        // `currentColor` with no `color` set is black, as in a browser.
        let image = load_default(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">
                <rect width="2" height="2" fill="currentColor"/>
            </svg>"#,
        );
        assert_eq!(rgba(&image, 0, 0), [0, 0, 0, 255]);

        // The file's own `color` and CSS apply.
        let image = load_default(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1" color="red">
                <style>.green { fill: #00ff00 }</style>
                <rect width="1" height="1" fill="currentColor"/>
                <rect x="1" width="1" height="1" class="green"/>
            </svg>"#,
        );
        assert_eq!(rgba(&image, 0, 0), [255, 0, 0, 255]);
        assert_eq!(rgba(&image, 1, 0), [0, 255, 0, 255]);
    }

    #[test]
    fn colors_are_srgb() {
        let image = load_default(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1">
                <rect width="1" height="1" fill="#808080"/>
            </svg>"##,
        );
        assert_eq!(rgba(&image, 0, 0), [128, 128, 128, 255]);
        let color = image.get_color_at(0, 0).unwrap();
        let expected = Color::srgb_u8(128, 128, 128).to_linear().to_f32_array();
        let actual = color.to_linear().to_f32_array();
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-3);
        }
    }

    #[test]
    fn default_sampler_is_linear() {
        assert!(matches!(
            load_default(HALF_RED).sampler,
            ImageSampler::Descriptor(descriptor)
                if descriptor.mipmap_filter == crate::ImageFilterMode::Linear
        ));
    }

    #[test]
    fn image_elements_are_not_resolved() {
        // A real, readable PNG on disk, which the default resolver would read.
        let mut red = tiny_skia::Pixmap::new(2, 2).unwrap();
        red.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let png_path = std::env::temp_dir().join(format!(
            "bevy_image_svg_external_{}.png",
            std::process::id()
        ));
        red.save_png(&png_path).unwrap();

        let href = png_path.to_str().unwrap();
        let path = Path::new("test.svg");
        let options = usvg_options(path);
        let resolved = (options.image_href_resolver.resolve_string)(href, &options);
        let default_resolved = (usvg::ImageHrefResolver::default_string_resolver())(href, &options);
        std::fs::remove_file(&png_path).ok();

        // The default resolver would have read the file; ours must not.
        assert!(default_resolved.is_some());
        assert!(resolved.is_none());

        // Embedded data, including nested SVGs, is not resolved either.
        for mime in ["image/png", "image/svg+xml"] {
            let resolved_data = (options.image_href_resolver.resolve_data)(
                mime,
                alloc::sync::Arc::new(Vec::new()),
                &options,
            );
            assert!(resolved_data.is_none(), "{mime}");
        }
    }

    #[test]
    fn image_elements_draw_nothing() {
        let image = load_default(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">
                <image href="data:image/svg+xml;utf8,&lt;svg xmlns='http://www.w3.org/2000/svg' width='2' height='2'&gt;&lt;rect width='2' height='2' fill='red'/&gt;&lt;/svg&gt;" width="2" height="2"/>
            </svg>"#,
        );
        assert_eq!(rgba(&image, 0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn settings_round_trip_through_serde() {
        let settings = SvgLoaderSettings {
            size: SvgRasterSize::Fit {
                width: 32,
                height: 16,
            },
            generate_mipmap: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let back: SvgLoaderSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.size, settings.size);
        assert_eq!(back.generate_mipmap, settings.generate_mipmap);
    }

    // SVG document tests

    fn svg(source: &str) -> Svg {
        Svg::parse(source.as_bytes(), Path::new("test.svg")).unwrap()
    }

    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];

    /// A 20x10 document: a blue square on the left, and a red one moved right by its parent.
    const TWO_SQUARES: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10">
        <rect id="blue" width="10" height="10" fill="blue"/>
        <g id="moved" transform="translate(10 0)">
            <rect id="red" width="10" height="10" fill="red"/>
        </g>
    </svg>"#;

    #[test]
    fn document_matches_the_image_loader() {
        let bytes = TWO_SQUARES.as_bytes();
        let path = Path::new("test.svg");
        let size = SvgRasterSize::Scale(2.0);
        let from_asset = Svg::parse(bytes, path).unwrap().rasterize(size).unwrap();
        let settings = crate::SvgLoaderSettings {
            size,
            ..Default::default()
        };
        let from_loader = crate::svg::rasterize(bytes, &settings, path).unwrap();
        assert_eq!(from_asset.size(), from_loader.size());
        assert_eq!(from_asset.data, from_loader.data);
    }

    #[test]
    fn element_is_cropped_to_its_bounds() {
        let svg = svg(TWO_SQUARES);
        let image = svg
            .rasterize_by_id("blue", SvgRasterSize::Intrinsic)
            .unwrap();
        assert_eq!((image.width(), image.height()), (10, 10));
        assert_eq!(rgba(&image, 0, 0), BLUE);
        assert_eq!(rgba(&image, 9, 9), BLUE);
    }

    #[test]
    fn ancestor_transforms_apply() {
        let svg = svg(TWO_SQUARES);
        for id in ["red", "moved"] {
            let image = svg.rasterize_by_id(id, SvgRasterSize::Intrinsic).unwrap();
            assert_eq!((image.width(), image.height()), (10, 10), "{id}");
            assert_eq!(rgba(&image, 0, 0), RED, "{id}");
            assert_eq!(rgba(&image, 9, 9), RED, "{id}");
        }

        let scaled = svg_with_scaled_parent();
        let image = scaled
            .rasterize_by_id("dot", SvgRasterSize::Intrinsic)
            .unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(rgba(&image, 0, 0), RED);
        assert_eq!(rgba(&image, 7, 7), RED);
    }

    fn svg_with_scaled_parent() -> Svg {
        svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
            <g transform="translate(4 4) scale(2)">
                <rect id="dot" width="4" height="4" fill="red"/>
            </g>
        </svg>"#,
        )
    }

    #[test]
    fn element_sizes_follow_the_request() {
        let image = svg(TWO_SQUARES)
            .rasterize_by_id("red", SvgRasterSize::Height(32))
            .unwrap();
        assert_eq!((image.width(), image.height()), (32, 32));
        assert_eq!(rgba(&image, 31, 31), RED);
    }

    #[test]
    fn missing_elements_are_an_error() {
        let svg = svg(TWO_SQUARES);
        assert!(svg.contains_element_by_id("red"));
        assert!(!svg.contains_element_by_id("green"));
        assert!(matches!(
            svg.rasterize_by_id("green", SvgRasterSize::Intrinsic),
            Err(SvgError::NotFound(id)) if id == "green"
        ));
    }

    #[test]
    fn size_is_the_declared_size() {
        assert_eq!(svg(TWO_SQUARES).size(), Vec2::new(20.0, 10.0));
    }
}
