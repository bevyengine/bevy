use bevy_image::TRANSPARENT_IMAGE_HANDLE;
use bevy_text::InlineBox;

/// An inline image
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(InlineBox)]
pub struct InlineImage {
    /// The tint color used to draw the image.
    ///
    /// This is multiplied by the color of each pixel in the image.
    /// The field value defaults to solid white, which will pass the image through unmodified.
    pub color: Color,
    /// Handle to the texture.
    ///
    /// This defaults to a [`TRANSPARENT_IMAGE_HANDLE`], which points to a fully transparent 1x1 texture.
    pub image: Handle<Image>,
}

impl Default for InlineImage {
    /// A transparent 1x1 image with a solid white tint.
    fn default() -> Self {
        ImageNode {
            // This should be white because the tint is multiplied with the image,
            // so if you set an actual image with default tint you'd want its original colors
            color: Color::WHITE,
            // This texture needs to be transparent by default, to avoid covering the background color
            image: TRANSPARENT_IMAGE_HANDLE,
        }
    }
}

/// For each `InlineImage` update the size of its `InlineBox` with its image size, if it changed.
pub fn update_inline_image_boxes(
    image_assets: Res<Assets<Image>>,
    mut query: Query<(
        &InlineImage,
        &mut InlineBox,
        Ref<ComputedUiRenderTargetInfo>,
    )>,
) {
    for (inline_image, mut inline_box) in &mut query {
        if let Some(image_asset) = image_assets.get(&inline_image.image) {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::InFlow,
                size: image_asset.size,
            });
        } else {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::OutOfFlow,
                size: Vec2::ZERO,
            });
        }
    }
}
