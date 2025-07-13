use super::TextureSlice;
use crate::{ExtractedSlice, Sprite, SpriteImageMode, TextureAtlasLayout};
use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::{Rect, Vec2};
use bevy_platform::collections::HashSet;

/// Component storing texture slices for tiled or sliced sprite entities
///
/// This component is automatically inserted and updated
#[derive(Debug, Clone, Component)]
pub struct ComputedTextureSlices(Vec<TextureSlice>);

impl ComputedTextureSlices {
    /// Computes [`ExtractedSlice`] iterator from the sprite slices
    ///
    /// # Arguments
    ///
    /// * `sprite` - The sprite component
    #[must_use]
    pub(crate) fn extract_slices<'a>(
        &'a self,
        sprite: &'a Sprite,
        anchor: Vec2,
    ) -> impl ExactSizeIterator<Item = ExtractedSlice> + 'a {
        let mut flip = Vec2::ONE;
        if sprite.flip_x {
            flip.x *= -1.0;
        }
        if sprite.flip_y {
            flip.y *= -1.0;
        }
        let anchor = anchor
            * sprite
                .custom_size
                .unwrap_or(sprite.rect.unwrap_or_default().size());
        self.0.iter().map(move |slice| ExtractedSlice {
            offset: slice.offset * flip - anchor,
            rect: slice.texture_rect,
            size: slice.draw_size,
        })
    }
}

/// Generates sprite slices for a [`Sprite`] with [`SpriteImageMode::Sliced`] or [`SpriteImageMode::Sliced`]. The slices
/// will be computed according to the `image_handle` dimensions or the sprite rect.
///
/// Returns `None` if the image asset is not loaded
///
/// # Arguments
///
/// * `sprite` - The sprite component with the image handle and image mode
/// * `images` - The image assets, use to retrieve the image dimensions
/// * `atlas_layouts` - The atlas layout assets, used to retrieve the texture atlas section rect
#[must_use]
fn compute_sprite_slices(
    sprite: &Sprite,
    images: &Assets<Image>,
    atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<ComputedTextureSlices> {
    let (image_size, texture_rect) = match &sprite.texture_atlas {
        Some(a) => {
            let layout = atlas_layouts.get(&a.layout)?;
            (
                layout.size.as_vec2(),
                layout.textures.get(a.index)?.as_rect(),
            )
        }
        None => {
            let image = images.get(&sprite.image)?;
            let size = Vec2::new(
                image.texture_descriptor.size.width as f32,
                image.texture_descriptor.size.height as f32,
            );
            let rect = sprite.rect.unwrap_or(Rect {
                min: Vec2::ZERO,
                max: size,
            });
            (size, rect)
        }
    };
    let slices = match &sprite.image_mode {
        SpriteImageMode::Sliced(slicer) => slicer.compute_slices(texture_rect, sprite.custom_size),
        SpriteImageMode::Tiled {
            tile_x,
            tile_y,
            stretch_value,
        } => {
            let slice = TextureSlice {
                texture_rect,
                draw_size: sprite.custom_size.unwrap_or(image_size),
                offset: Vec2::ZERO,
            };
            slice.tiled(*stretch_value, (*tile_x, *tile_y))
        }
        SpriteImageMode::Auto => {
            unreachable!("Slices should not be computed for SpriteImageMode::Stretch")
        }
        SpriteImageMode::Scale(_) => {
            unreachable!("Slices should not be computed for SpriteImageMode::Scale")
        }
    };
    Some(ComputedTextureSlices(slices))
}

/// System reacting to added or modified [`Image`] handles, and recompute sprite slices
/// on sprite entities with a matching  [`SpriteImageMode`]
pub(crate) fn compute_slices_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    sprites: Query<(Entity, &Sprite)>,
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
    for (entity, sprite) in &sprites {
        if !sprite.image_mode.uses_slices() {
            continue;
        }
        if !added_handles.contains(&sprite.image.id()) {
            continue;
        }
        if let Some(slices) = compute_sprite_slices(sprite, &images, &atlas_layouts) {
            commands.entity(entity).insert(slices);
        }
    }
}

/// System reacting to changes on the [`Sprite`] component to compute the sprite slices
pub(crate) fn compute_slices_on_sprite_change(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    changed_sprites: Query<(Entity, &Sprite), Changed<Sprite>>,
) {
    for (entity, sprite) in &changed_sprites {
        if !sprite.image_mode.uses_slices() {
            continue;
        }
        if let Some(slices) = compute_sprite_slices(sprite, &images, &atlas_layouts) {
            commands.entity(entity).insert(slices);
        }
    }
}
