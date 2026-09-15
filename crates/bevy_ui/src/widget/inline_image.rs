use bevy_asset::asset_changed::AssetChanged;
use bevy_asset::AsAssetId;
use bevy_asset::Assets;
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Or;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_image::Image;
use bevy_image::TRANSPARENT_IMAGE_HANDLE;
use bevy_math::Vec2;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_text::InlineBox;

/// An inline image
#[derive(Component, Debug, Clone, Reflect, PartialEq)]
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
    /// Set a fixed width in logical pixels for the image's inline box target.
    pub width: Option<f32>,
    /// Set a fixed height in logical pixels for the image's inline box target.
    pub height: Option<f32>,
    /// Flip the image on the x-axis.
    pub flip_x: bool,
    /// Flip the image on the y-axis.
    pub flip_y: bool,
}

impl Default for InlineImage {
    /// A transparent 1x1 image with a solid white tint.
    fn default() -> Self {
        InlineImage {
            color: Color::WHITE,
            image: TRANSPARENT_IMAGE_HANDLE,
            width: None,
            height: None,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl InlineImage {
    /// Resolve the inline box size. Preserves the image's aspect ratio if dimensions unset.
    pub fn resolve_inline_box_size(&self, image_size: Vec2) -> Vec2 {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Vec2::new(w, h),
            (Some(w), None) => Vec2::new(w, image_size.y * (w / image_size.x)),
            (None, Some(h)) => Vec2::new(image_size.x * (h / image_size.y), h),
            _ => image_size,
        }
    }
}

impl AsAssetId for InlineImage {
    type Asset = Image;

    fn as_asset_id(&self) -> bevy_asset::AssetId<Self::Asset> {
        self.image.id()
    }
}

/// For each `InlineImage` update the size of its `InlineBox` with its image size, if it changed.
pub fn update_inline_image_boxes(
    image_assets: Res<Assets<Image>>,
    mut query: Query<
        (&InlineImage, &mut InlineBox),
        Or<(Changed<InlineImage>, AssetChanged<InlineImage>)>,
    >,
) {
    for (inline_image, mut inline_box) in &mut query {
        if let Some(size) = image_assets
            .get(&inline_image.image)
            .map(|image_asset| inline_image.resolve_inline_box_size(image_asset.size().as_vec2()))
            .filter(|size| size.is_finite() && size.cmpgt(Vec2::ZERO).all())
        {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::InFlow,
                size,
            });
        } else {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::OutOfFlow,
                size: Vec2::ZERO,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_inline_box_size() {
        let image = InlineImage {
            width: Some(20.),
            height: Some(10.),
            ..Default::default()
        };
        assert_eq!(
            image.resolve_inline_box_size(Vec2::new(5., 10.)),
            Vec2::new(20., 10.)
        );

        let image = InlineImage {
            width: Some(10.),
            height: None,
            ..Default::default()
        };
        assert_eq!(
            image.resolve_inline_box_size(Vec2::new(5., 10.)),
            Vec2::new(10., 20.)
        );

        let image = InlineImage {
            width: None,
            height: Some(10.),
            ..Default::default()
        };
        assert_eq!(
            image.resolve_inline_box_size(Vec2::new(5., 10.)),
            Vec2::new(5., 10.)
        );

        let image = InlineImage {
            width: None,
            height: None,
            ..Default::default()
        };
        assert_eq!(
            image.resolve_inline_box_size(Vec2::new(5., 10.)),
            Vec2::new(5., 10.)
        );
    }
}
