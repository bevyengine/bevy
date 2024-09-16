use crate::{ExtractedSprite, ImageScaleMode, Sprite, TextureAtlas, TextureAtlasLayout};

use super::TextureSlice;
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_render::texture::Image;
use bevy_transform::prelude::*;
use bevy_utils::HashSet;

/// Component storing texture slices for sprite entities with a [`ImageScaleMode`]
///
/// This component is automatically inserted and updated
#[derive(Debug, Clone, Component)]
pub struct ComputedTextureSlices(Vec<TextureSlice>);

impl ComputedTextureSlices {
    /// Computes [`ExtractedSprite`] iterator from the sprite slices
    ///
    /// # Arguments
    ///
    /// * `transform` - the sprite entity global transform
    /// * `original_entity` - the sprite entity
    /// * `sprite` - The sprite component
    /// * `handle` - The sprite texture handle
    #[must_use]
    pub(crate) fn extract_sprites<'a>(
        &'a self,
        transform: &'a GlobalTransform,
        original_entity: Entity,
        sprite: &'a Sprite,
        handle: &'a Handle<Image>,
    ) -> impl ExactSizeIterator<Item = ExtractedSprite> + 'a {
        let mut flip = Vec2::ONE;
        let [mut flip_x, mut flip_y] = [false; 2];
        if sprite.flip_x {
            flip.x *= -1.0;
            flip_x = true;
        }
        if sprite.flip_y {
            flip.y *= -1.0;
            flip_y = true;
        }
        self.0.iter().map(move |slice| {
            let offset = (slice.offset * flip).extend(0.0);
            let transform = transform.mul_transform(Transform::from_translation(offset));
            ExtractedSprite {
                original_entity: Some(original_entity),
                color: sprite.color.into(),
                transform,
                rect: Some(slice.texture_rect),
                custom_size: Some(slice.draw_size),
                flip_x,
                flip_y,
                image_handle_id: handle.id(),
                anchor: Self::redepend_anchor_from_sprite_to_slice(sprite, slice),
            }
        })
    }

    fn redepend_anchor_from_sprite_to_slice(sprite: &Sprite, slice: &TextureSlice) -> Vec2 {
        let sprite_size = sprite
            .custom_size
            .unwrap_or(sprite.rect.unwrap_or_default().size());
        if sprite_size == Vec2::ZERO {
            sprite.anchor.as_vec()
        } else {
            sprite.anchor.as_vec() * sprite_size / slice.draw_size
        }
    }
}

/// Generates sprite slices for a `sprite` given a `scale_mode`. The slices
/// will be computed according to the `image_handle` dimensions or the sprite rect.
///
/// Returns `None` if the image asset is not loaded
///
/// # Arguments
///
/// * `sprite` - The sprite component, will be used to find the draw area size
/// * `scale_mode` - The image scaling component
/// * `image_handle` - The texture to slice or tile
/// * `images` - The image assets, use to retrieve the image dimensions
/// * `atlas` - Optional texture atlas, if set the slicing will happen on the matching sub section
///     of the texture
/// * `atlas_layouts` - The atlas layout assets, used to retrieve the texture atlas section rect
#[must_use]
fn compute_sprite_slices(
    sprite: &Sprite,
    scale_mode: &ImageScaleMode,
    image_handle: &Handle<Image>,
    images: &Assets<Image>,
    atlas: Option<&TextureAtlas>,
    atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<ComputedTextureSlices> {
    let (image_size, texture_rect) = match atlas {
        Some(a) => {
            let layout = atlas_layouts.get(&a.layout)?;
            (
                layout.size.as_vec2(),
                layout.textures.get(a.index)?.as_rect(),
            )
        }
        None => {
            let image = images.get(image_handle)?;
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
    let slices = match scale_mode {
        ImageScaleMode::Sliced(slicer) => slicer.compute_slices(texture_rect, sprite.custom_size),
        ImageScaleMode::Tiled {
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
    };
    Some(ComputedTextureSlices(slices))
}

/// System reacting to added or modified [`Image`] handles, and recompute sprite slices
/// on matching sprite entities with a [`ImageScaleMode`] component
pub(crate) fn compute_slices_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    sprites: Query<(
        Entity,
        &ImageScaleMode,
        &Sprite,
        &Handle<Image>,
        Option<&TextureAtlas>,
    )>,
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
    for (entity, scale_mode, sprite, image_handle, atlas) in &sprites {
        if !added_handles.contains(&image_handle.id()) {
            continue;
        }
        if let Some(slices) = compute_sprite_slices(
            sprite,
            scale_mode,
            image_handle,
            &images,
            atlas,
            &atlas_layouts,
        ) {
            commands.entity(entity).insert(slices);
        }
    }
}

/// System reacting to changes on relevant sprite bundle components to compute the sprite slices
/// on matching sprite entities with a [`ImageScaleMode`] component
pub(crate) fn compute_slices_on_sprite_change(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    changed_sprites: Query<
        (
            Entity,
            &ImageScaleMode,
            &Sprite,
            &Handle<Image>,
            Option<&TextureAtlas>,
        ),
        Or<(
            Changed<ImageScaleMode>,
            Changed<Handle<Image>>,
            Changed<Sprite>,
            Changed<TextureAtlas>,
        )>,
    >,
) {
    for (entity, scale_mode, sprite, image_handle, atlas) in &changed_sprites {
        if let Some(slices) = compute_sprite_slices(
            sprite,
            scale_mode,
            image_handle,
            &images,
            atlas,
            &atlas_layouts,
        ) {
            commands.entity(entity).insert(slices);
        }
    }
}
