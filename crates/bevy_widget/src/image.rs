use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
#[cfg(feature = "bevy_text")]
use bevy_ecs::query::Without;
use bevy_ecs::{
    prelude::{Component, Bundle},
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res}, schedule::IntoSystemConfigs,
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect, ReflectFromReflect};
use bevy_render::{texture::Image, view::{Visibility, ComputedVisibility}};
#[cfg(feature = "bevy_text")]
use bevy_text::Text;
use bevy_transform::prelude::{Transform, GlobalTransform};
use bevy_ui::{measurement::AvailableSpace, ContentSize, Measure, Node, UiImage, UiSystem, Style, BackgroundColor, FocusPolicy, ZIndex};

use crate::text_system;

/// The size of the image in physical pixels
///
/// This field is set automatically by `update_image_calculated_size_system`
#[derive(Component, Debug, Copy, Clone, Default, Reflect, FromReflect)]
#[reflect(Component, Default, FromReflect)]
pub struct UiImageSize {
    size: Vec2,
}

impl UiImageSize {
    pub fn size(&self) -> Vec2 {
        self.size
    }
}

#[derive(Clone)]
pub struct ImageMeasure {
    // target size of the image
    size: Vec2,
}

impl Measure for ImageMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        let mut size = self.size;
        match (width, height) {
            (None, None) => {}
            (Some(width), None) => {
                size.y = width * size.y / size.x;
                size.x = width;
            }
            (None, Some(height)) => {
                size.x = height * size.x / size.y;
                size.y = height;
            }
            (Some(width), Some(height)) => {
                size.x = width;
                size.y = height;
            }
        }
        size
    }
}

/// Updates content size of the node based on the image provided
pub fn update_image_content_size_system(
    textures: Res<Assets<Image>>,
    #[cfg(feature = "bevy_text")] mut query: Query<
        (&mut ContentSize, &UiImage, &mut UiImageSize),
        (With<Node>, Without<Text>),
    >,
    #[cfg(not(feature = "bevy_text"))] mut query: Query<
        (&mut ContentSize, &UiImage, &mut UiImageSize),
        With<Node>,
    >,
) {
    for (mut content_size, image, mut image_size) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Vec2::new(
                texture.texture_descriptor.size.width as f32,
                texture.texture_descriptor.size.height as f32,
            );
            // Update only if size has changed to avoid needless layout calculations
            if size != image_size.size {
                image_size.size = size;
                content_size.set(ImageMeasure { size });
            }
        }
    }
}

/// A plugin for image widgets
#[derive(Default)]
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiImageSize>().add_systems(PostUpdate, {
            let system = update_image_content_size_system.before(UiSystem::Layout);
            // Potential conflicts: `Assets<Image>`
            // They run independently since `widget::image_node_system` will only ever observe
            // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
            // will never modify a pre-existing `Image` asset.
            #[cfg(feature = "bevy_text")]
            let system = system
                .ambiguous_with(bevy_text::update_text2d_layout)
                .ambiguous_with(text_system);

            system
        });
    }
}

/// A UI node that is an image
#[derive(Bundle, Debug, Default)]
pub struct ImageBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// The calculated size based on the given image
    pub calculated_size: ContentSize,
    /// The background color, which serves as a "fill" for this node
    ///
    /// Combines with `UiImage` to tint the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}
