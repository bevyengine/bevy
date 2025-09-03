use crate::{ComputedUiRenderTargetInfo, ContentSize, Measure, MeasureArgs, Node, NodeMeasure};
use bevy_asset::{AsAssetId, AssetId, Assets, Handle};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::{prelude::*, TRANSPARENT_IMAGE_HANDLE};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_sprite::TextureSlicer;
use taffy::{MaybeMath, MaybeResolve};

/// A UI Node that renders an image.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Node, ImageNodeSize, ContentSize)]
pub struct ImageNode {
    /// The tint color used to draw the image.
    ///
    /// This is multiplied by the color of each pixel in the image.
    /// The field value defaults to solid white, which will pass the image through unmodified.
    pub color: Color,
    /// Handle to the texture.
    ///
    /// This defaults to a [`TRANSPARENT_IMAGE_HANDLE`], which points to a fully transparent 1x1 texture.
    pub image: Handle<Image>,
    /// The (optional) texture atlas used to render the image.
    pub texture_atlas: Option<TextureAtlas>,
    /// Whether the image should be flipped along its x-axis.
    pub flip_x: bool,
    /// Whether the image should be flipped along its y-axis.
    pub flip_y: bool,
    /// An optional rectangle representing the region of the image to render, instead of rendering
    /// the full image. This is an easy one-off alternative to using a [`TextureAtlas`].
    ///
    /// When used with a [`TextureAtlas`], the rect
    /// is offset by the atlas's minimal (top-left) corner position.
    pub rect: Option<Rect>,
    /// Controls how the image is altered to fit within the layout and how the layout algorithm determines the space to allocate for the image.
    pub image_mode: NodeImageMode,
}

impl Default for ImageNode {
    /// A transparent 1x1 image with a solid white tint.
    ///
    /// # Warning
    ///
    /// This will be invisible by default.
    /// To set this to a visible image, you need to set the `texture` field to a valid image handle,
    /// or use [`Handle<Image>`]'s default 1x1 solid white texture (as is done in [`ImageNode::solid_color`]).
    fn default() -> Self {
        ImageNode {
            // This should be white because the tint is multiplied with the image,
            // so if you set an actual image with default tint you'd want its original colors
            color: Color::WHITE,
            texture_atlas: None,
            // This texture needs to be transparent by default, to avoid covering the background color
            image: TRANSPARENT_IMAGE_HANDLE,
            flip_x: false,
            flip_y: false,
            rect: None,
            image_mode: NodeImageMode::Auto,
        }
    }
}

impl ImageNode {
    /// Create a new [`ImageNode`] with the given texture.
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            image: texture,
            color: Color::WHITE,
            ..Default::default()
        }
    }

    /// Create a solid color [`ImageNode`].
    ///
    /// This is primarily useful for debugging / mocking the extents of your image.
    pub fn solid_color(color: Color) -> Self {
        Self {
            image: Handle::default(),
            color,
            flip_x: false,
            flip_y: false,
            texture_atlas: None,
            rect: None,
            image_mode: NodeImageMode::Auto,
        }
    }

    /// Create a [`ImageNode`] from an image, with an associated texture atlas
    pub fn from_atlas_image(image: Handle<Image>, atlas: TextureAtlas) -> Self {
        Self {
            image,
            texture_atlas: Some(atlas),
            ..Default::default()
        }
    }

    /// Set the color tint
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Flip the image along its x-axis
    #[must_use]
    pub const fn with_flip_x(mut self) -> Self {
        self.flip_x = true;
        self
    }

    /// Flip the image along its y-axis
    #[must_use]
    pub const fn with_flip_y(mut self) -> Self {
        self.flip_y = true;
        self
    }

    #[must_use]
    pub const fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    #[must_use]
    pub const fn with_mode(mut self, mode: NodeImageMode) -> Self {
        self.image_mode = mode;
        self
    }
}

impl From<Handle<Image>> for ImageNode {
    fn from(texture: Handle<Image>) -> Self {
        Self::new(texture)
    }
}

impl AsAssetId for ImageNode {
    type Asset = Image;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.image.id()
    }
}

/// Controls how the image is altered to fit within the layout and how the layout algorithm determines the space in the layout for the image
#[derive(Default, Debug, Clone, PartialEq, Reflect)]
#[reflect(Clone, Default, PartialEq)]
pub enum NodeImageMode {
    /// The image will be sized automatically by taking the size of the source image and applying any layout constraints.
    #[default]
    Auto,
    /// The image will be resized to match the size of the node. The image's original size and aspect ratio will be ignored.
    Stretch,
    /// The texture will be cut in 9 slices, keeping the texture in proportions on resize
    Sliced(TextureSlicer),
    /// The texture will be repeated if stretched beyond `stretched_value`
    Tiled {
        /// Should the image repeat horizontally
        tile_x: bool,
        /// Should the image repeat vertically
        tile_y: bool,
        /// The texture will repeat when the ratio between the *drawing dimensions* of texture and the
        /// *original texture size* are above this value.
        stretch_value: f32,
    },
}

impl NodeImageMode {
    /// Returns true if this mode uses slices internally ([`NodeImageMode::Sliced`] or [`NodeImageMode::Tiled`])
    #[inline]
    pub fn uses_slices(&self) -> bool {
        matches!(
            self,
            NodeImageMode::Sliced(..) | NodeImageMode::Tiled { .. }
        )
    }
}

/// The size of the image's texture
///
/// This component is updated automatically by [`update_image_content_size_system`]
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ImageNodeSize {
    /// The size of the image's texture
    ///
    /// This field is updated automatically by [`update_image_content_size_system`]
    size: UVec2,
}

impl ImageNodeSize {
    /// The size of the image's texture
    pub fn size(&self) -> UVec2 {
        self.size
    }
}

#[derive(Clone)]
/// Used to calculate the size of UI image nodes
pub struct ImageMeasure {
    /// The size of the image's texture
    pub size: Vec2,
}

impl Measure for ImageMeasure {
    fn measure(&mut self, measure_args: MeasureArgs, style: &taffy::Style) -> Vec2 {
        let MeasureArgs {
            width,
            height,
            available_width,
            available_height,
            ..
        } = measure_args;

        // Convert available width/height into an option
        let parent_width = available_width.into_option();
        let parent_height = available_height.into_option();

        // Resolve styles
        let s_aspect_ratio = style.aspect_ratio;
        let s_width = style.size.width.maybe_resolve(parent_width);
        let s_min_width = style.min_size.width.maybe_resolve(parent_width);
        let s_max_width = style.max_size.width.maybe_resolve(parent_width);
        let s_height = style.size.height.maybe_resolve(parent_height);
        let s_min_height = style.min_size.height.maybe_resolve(parent_height);
        let s_max_height = style.max_size.height.maybe_resolve(parent_height);

        // Determine width and height from styles and known_sizes (if a size is available
        // from any of these sources)
        let width = width.or(s_width
            .or(s_min_width)
            .maybe_clamp(s_min_width, s_max_width));
        let height = height.or(s_height
            .or(s_min_height)
            .maybe_clamp(s_min_height, s_max_height));

        // Use aspect_ratio from style, fall back to inherent aspect ratio
        let aspect_ratio = s_aspect_ratio.unwrap_or_else(|| self.size.x / self.size.y);

        // Apply aspect ratio
        // If only one of width or height was determined at this point, then the other is set beyond this point using the aspect ratio.
        let taffy_size = taffy::Size { width, height }.maybe_apply_aspect_ratio(Some(aspect_ratio));

        // Use computed sizes or fall back to image's inherent size
        Vec2 {
            x: taffy_size
                .width
                .unwrap_or(self.size.x)
                .maybe_clamp(s_min_width, s_max_width),
            y: taffy_size
                .height
                .unwrap_or(self.size.y)
                .maybe_clamp(s_min_height, s_max_height),
        }
    }
}

type UpdateImageFilter = (With<Node>, Without<crate::prelude::Text>);

/// Updates content size of the node based on the image provided
pub fn update_image_content_size_system(
    textures: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<
        (
            &mut ContentSize,
            Ref<ImageNode>,
            &mut ImageNodeSize,
            Ref<ComputedUiRenderTargetInfo>,
        ),
        UpdateImageFilter,
    >,
) {
    for (mut content_size, image, mut image_size, computed_target) in &mut query {
        if !matches!(image.image_mode, NodeImageMode::Auto)
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
        {
            if image.is_changed() {
                // Mutably derefs, marking the `ContentSize` as changed ensuring `ui_layout_system` will remove the node's measure func if present.
                content_size.measure = None;
            }
            continue;
        }

        if let Some(size) =
            image
                .rect
                .map(|rect| rect.size().as_uvec2())
                .or_else(|| match &image.texture_atlas {
                    Some(atlas) => atlas.texture_rect(&atlases).map(|t| t.size()),
                    None => textures.get(&image.image).map(Image::size),
                })
        {
            // Update only if size or scale factor has changed to avoid needless layout calculations
            if size != image_size.size || computed_target.is_changed() || content_size.is_added() {
                image_size.size = size;
                content_size.set(NodeMeasure::Image(ImageMeasure {
                    // multiply the image size by the scale factor to get the physical size
                    size: size.as_vec2() * computed_target.scale_factor(),
                }));
            }
        }
    }
}
