//! Utility types and systems for rendering of image nodes.

use bevy_asset::{asset_changed::AssetChanged, AsAssetId, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut as _,
    component::Component,
    entity::Entity,
    query::{Changed, Or},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_image::TextureAtlasLayout;
use bevy_reflect::Reflect;
use bevy_ui::widget::ImageNode;

// This component is more or less a workaround for the fact that `AsAssetId`
// only allows each component to expose one asset. `ImageNode` exposes two types
// of assets: an `Image` and a `TextureAtlasLayout`. We have to mark the image
// node as changed if either one of those assets changes. The only way to detect
// asset changes is to use the `AssetChanged` query filter. Unfortunately, the
// `AssetChanged` query filter relies on `AsAssetId`, which we can only
// implement once per component. Thus we need this second component, which
// essentially serves to provide a second implementation of `AsAssetId` on
// `ImageNode`.

/// The texture atlas layout, if the image has one.
///
/// The [`update_texture_atlas_layout_components`] system automatically keeps
/// this component up to date based on [`ImageNode::texture_atlas`]. Don't
/// update this component yourself; [`ImageNode::texture_atlas`] is the source
/// of truth.
#[derive(Component, Debug, Clone, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Clone)]
pub(crate) struct ImageNodeTextureAtlasLayout(Handle<TextureAtlasLayout>);

impl AsAssetId for ImageNodeTextureAtlasLayout {
    type Asset = TextureAtlasLayout;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// A system that marks [`ImageNode`]s as changed if either their
/// [`bevy_image::Image`] or [`TextureAtlasLayout`] changed.
pub(crate) fn mark_images_as_changed_if_their_assets_changed(
    mut query: Query<
        (&mut ImageNode, Option<&mut ImageNodeTextureAtlasLayout>),
        Or<(
            AssetChanged<ImageNode>,
            AssetChanged<ImageNodeTextureAtlasLayout>,
        )>,
    >,
) {
    for (mut image, maybe_texture_atlas_layout) in &mut query {
        image.set_changed();
        if let Some(mut texture_atlas_layout) = maybe_texture_atlas_layout {
            texture_atlas_layout.set_changed();
        }
    }
}

/// A system that copies the [`TextureAtlasLayout`] stored within an
/// [`ImageNode`] to the [`TextureAtlasLayout`] component.
pub(crate) fn update_texture_atlas_layout_components(
    mut commands: Commands,
    images_query: Query<(Entity, &ImageNode), Changed<ImageNode>>,
) {
    for (entity, image_node) in &images_query {
        match image_node.texture_atlas {
            Some(ref texture_atlas) => {
                commands
                    .entity(entity)
                    .insert(ImageNodeTextureAtlasLayout(texture_atlas.layout.clone()));
            }
            None => {
                commands
                    .entity(entity)
                    .remove::<ImageNodeTextureAtlasLayout>();
            }
        }
    }
}
