// This module is mostly copied and pasted from `bevy_sprite::texture_slice`
//
// A more centralized solution should be investigated in the future

use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_render::texture::Image;
use bevy_sprite::{ImageScaleMode, TextureSlice};
use bevy_transform::prelude::*;
use bevy_utils::HashSet;

use crate::{BackgroundColor, CalculatedClip, ExtractedUiNode, Node, UiImage};

/// Component storing texture slices for image nodes entities with a tiled or sliced  [`ImageScaleMode`]
///
/// This component is automatically inserted and updated
#[derive(Debug, Clone, Component)]
pub struct ComputedTextureSlices {
    slices: Vec<TextureSlice>,
    image_size: Vec2,
}

impl ComputedTextureSlices {
    /// Computes [`ExtractedUiNode`] iterator from the sprite slices
    ///
    /// # Arguments
    ///
    /// * `transform` - the sprite entity global transform
    /// * `original_entity` - the sprite entity
    /// * `sprite` - The sprite component
    /// * `handle` - The sprite texture handle
    #[must_use]
    pub(crate) fn extract_ui_nodes<'a>(
        &'a self,
        transform: &'a GlobalTransform,
        node: &'a Node,
        background_color: &'a BackgroundColor,
        image: &'a UiImage,
        clip: Option<&'a CalculatedClip>,
        camera_entity: Entity,
    ) -> impl ExactSizeIterator<Item = ExtractedUiNode> + 'a {
        let mut flip = Vec2::new(1.0, -1.0);
        let [mut flip_x, mut flip_y] = [false; 2];
        if image.flip_x {
            flip.x *= -1.0;
            flip_x = true;
        }
        if image.flip_y {
            flip.y *= -1.0;
            flip_y = true;
        }
        self.slices.iter().map(move |slice| {
            let offset = (slice.offset * flip).extend(0.0);
            let transform = transform.mul_transform(Transform::from_translation(offset));
            let scale = slice.draw_size / slice.texture_rect.size();
            let mut rect = slice.texture_rect;
            rect.min *= scale;
            rect.max *= scale;
            let atlas_size = Some(self.image_size * scale);
            ExtractedUiNode {
                stack_index: node.stack_index,
                color: background_color.0,
                transform: transform.compute_matrix(),
                rect,
                flip_x,
                flip_y,
                image: image.texture.id(),
                atlas_size,
                clip: clip.map(|clip| clip.clip),
                camera_entity,
            }
        })
    }
}

/// Generates sprite slices for a `sprite` given a `scale_mode`. The slices
/// will be computed according to the `image_handle` dimensions or the sprite rect.
///
/// Returns `None` if the image asset is not loaded
#[must_use]
fn compute_texture_slices(
    draw_area: Vec2,
    scale_mode: &ImageScaleMode,
    image_handle: &UiImage,
    images: &Assets<Image>,
) -> Option<ComputedTextureSlices> {
    let image_size = images.get(&image_handle.texture).map(|i| {
        Vec2::new(
            i.texture_descriptor.size.width as f32,
            i.texture_descriptor.size.height as f32,
        )
    })?;
    let texture_rect = Rect {
        min: Vec2::ZERO,
        max: image_size,
    };
    let slices = match scale_mode {
        ImageScaleMode::Sliced(slicer) => slicer.compute_slices(texture_rect, Some(draw_area)),
        ImageScaleMode::Tiled {
            tile_x,
            tile_y,
            stretch_value,
        } => {
            let slice = TextureSlice {
                texture_rect,
                draw_size: draw_area,
                offset: Vec2::ZERO,
            };
            slice.tiled(*stretch_value, (*tile_x, *tile_y))
        }
    };
    Some(ComputedTextureSlices { slices, image_size })
}

/// System reacting to added or modified [`Image`] handles, and recompute sprite slices
/// on matching sprite entities with a [`ImageScaleMode`] component
pub(crate) fn compute_slices_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    ui_nodes: Query<(Entity, &ImageScaleMode, &Node, &UiImage)>,
) {
    // We store the asset ids of added/modified image assets
    let added_handles: HashSet<_> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    if added_handles.is_empty() {
        return;
    }
    // We recompute the sprite slices for sprite entities with a matching asset handle id
    for (entity, scale_mode, ui_node, image) in &ui_nodes {
        if !added_handles.contains(&image.texture.id()) {
            continue;
        }
        if let Some(slices) = compute_texture_slices(ui_node.size(), scale_mode, image, &images) {
            commands.entity(entity).insert(slices);
        }
    }
}

/// System reacting to changes on relevant sprite bundle components to compute the sprite slices
/// on matching sprite entities with a [`ImageScaleMode`] component
pub(crate) fn compute_slices_on_image_change(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    changed_nodes: Query<
        (Entity, &ImageScaleMode, &Node, &UiImage),
        Or<(Changed<ImageScaleMode>, Changed<UiImage>, Changed<Node>)>,
    >,
) {
    for (entity, scale_mode, ui_node, image) in &changed_nodes {
        if let Some(slices) = compute_texture_slices(ui_node.size(), scale_mode, image, &images) {
            commands.entity(entity).insert(slices);
        }
    }
}
