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
}

impl Default for InlineImage {
    /// A transparent 1x1 image with a solid white tint.
    fn default() -> Self {
        InlineImage {
            color: Color::WHITE,
            image: TRANSPARENT_IMAGE_HANDLE,
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
        if let Some(image_asset) = image_assets.get(&inline_image.image) {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::InFlow,
                size: image_asset.size().as_vec2(),
            });
        } else {
            inline_box.set_if_neq(InlineBox {
                kind: bevy_text::InlineBoxKind::OutOfFlow,
                size: Vec2::ZERO,
            });
        }
    }
}
