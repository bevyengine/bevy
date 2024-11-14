//! A [`bevy_picking`] backend for sprites. Works for simple sprites and sprite atlases. Works for
//! sprites with arbitrary transforms. Picking is done based on sprite bounds, not visible pixels.
//! This means a partially transparent sprite is pickable even in its transparent areas.

use core::cmp::Reverse;

use crate::{Sprite, TextureAtlasLayout};
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::{prelude::*, FloatExt, FloatOrd};
use bevy_picking::backend::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use bevy_window::PrimaryWindow;

/// Runtime settings for the [`SpritePickingPlugin`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct SpriteBackendSettings {
    /// When set to `true` picking will ignore any part of a sprite which is transparent
    /// Off by default for backwards compatibility. This setting is provided to give you fine-grained
    /// control over if transparency on sprites is ignored.
    pub transparency_passthrough: bool,
    /// How Opaque does part of a sprite need to be in order count as none-transparent (defaults to 10)
    ///
    /// This is on a scale from 0 - 255 representing the alpha channel value you'd get in most art programs.
    pub transparency_cutoff: u8,
}

impl Default for SpriteBackendSettings {
    fn default() -> Self {
        Self {
            transparency_passthrough: false,
            transparency_cutoff: 10,
        }
    }
}

#[derive(Clone)]
pub struct SpritePickingPlugin;

impl Plugin for SpritePickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteBackendSettings>()
            .add_systems(PreUpdate, sprite_picking.in_set(PickSet::Backend));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sprite_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &OrthographicProjection)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    texture_atlas_layout: Res<Assets<TextureAtlasLayout>>,
    settings: Res<SpriteBackendSettings>,
    sprite_query: Query<(
        Entity,
        &Sprite,
        &GlobalTransform,
        Option<&PickingBehavior>,
        &ViewVisibility,
    )>,
    mut output: EventWriter<PointerHits>,
) {
    let mut sorted_sprites: Vec<_> = sprite_query
        .iter()
        .filter_map(|(entity, sprite, transform, picking_behavior, vis)| {
            if !transform.affine().is_nan() && vis.get() {
                Some((entity, sprite, transform, picking_behavior))
            } else {
                None
            }
        })
        .collect();
    sorted_sprites.sort_by_key(|x| Reverse(FloatOrd(x.2.translation().z)));

    let primary_window = primary_window.get_single().ok();

    for (pointer, location) in pointers.iter().filter_map(|(pointer, pointer_location)| {
        pointer_location.location().map(|loc| (pointer, loc))
    }) {
        let mut blocked = false;
        let Some((cam_entity, camera, cam_transform, cam_ortho)) = cameras
            .iter()
            .filter(|(_, camera, _, _)| camera.is_active)
            .find(|(_, camera, _, _)| {
                camera
                    .target
                    .normalize(primary_window)
                    .map(|x| x == location.target)
                    .unwrap_or(false)
            })
        else {
            continue;
        };

        let Ok(cursor_ray_world) = camera.viewport_to_world(cam_transform, location.position)
        else {
            continue;
        };
        let cursor_ray_len = cam_ortho.far - cam_ortho.near;
        let cursor_ray_end = cursor_ray_world.origin + cursor_ray_world.direction * cursor_ray_len;

        let picks: Vec<(Entity, HitData)> = sorted_sprites
            .iter()
            .copied()
            .filter_map(|(entity, sprite, sprite_transform, picking_behavior)| {
                if blocked {
                    return None;
                }

                // Hit box in sprite coordinate system
                let extents = match (sprite.custom_size, &sprite.texture_atlas) {
                    (Some(custom_size), _) => custom_size,
                    (None, None) => images.get(&sprite.image)?.size().as_vec2(),
                    (None, Some(atlas)) => texture_atlas_layout
                        .get(&atlas.layout)
                        .and_then(|layout| layout.textures.get(atlas.index))
                        // Dropped atlas layouts and indexes out of bounds are rendered as a sprite
                        .map_or(images.get(&sprite.image)?.size().as_vec2(), |rect| {
                            rect.size().as_vec2()
                        }),
                };
                let anchor = sprite.anchor.as_vec();
                let center = -anchor * extents;
                let rect = Rect::from_center_half_size(center, extents / 2.0);

                // Transform cursor line segment to sprite coordinate system
                let world_to_sprite = sprite_transform.affine().inverse();
                let cursor_start_sprite = world_to_sprite.transform_point3(cursor_ray_world.origin);
                let cursor_end_sprite = world_to_sprite.transform_point3(cursor_ray_end);

                // Find where the cursor segment intersects the plane Z=0 (which is the sprite's
                // plane in sprite-local space). It may not intersect if, for example, we're
                // viewing the sprite side-on
                if cursor_start_sprite.z == cursor_end_sprite.z {
                    // Cursor ray is parallel to the sprite and misses it
                    return None;
                }
                let lerp_factor =
                    f32::inverse_lerp(cursor_start_sprite.z, cursor_end_sprite.z, 0.0);
                if !(0.0..=1.0).contains(&lerp_factor) {
                    // Lerp factor is out of range, meaning that while an infinite line cast by
                    // the cursor would intersect the sprite, the sprite is not between the
                    // camera's near and far planes
                    return None;
                }
                // Otherwise we can interpolate the xy of the start and end positions by the
                // lerp factor to get the cursor position in sprite space!
                let cursor_pos_sprite = cursor_start_sprite
                    .lerp(cursor_end_sprite, lerp_factor)
                    .xy();

                let is_cursor_in_sprite = rect.contains(cursor_pos_sprite);

                let cursor_in_valid_pixels_of_sprite = is_cursor_in_sprite
                    && (!settings.transparency_passthrough || {
                        let texture: &Image = images.get(&sprite.image)?;
                        // If using a texture atlas, grab the offset of the current sprite index. (0,0) otherwise
                        let texture_rect = sprite
                            .texture_atlas
                            .as_ref()
                            .and_then(|atlas| {
                                texture_atlas_layout
                                    .get(&atlas.layout)
                                    .map(|f| f.textures[atlas.index])
                            })
                            .or(Some(URect::new(0, 0, texture.width(), texture.height())))?;
                        // get mouse position on texture
                        let texture_position = (texture_rect.center().as_vec2()
                            + cursor_pos_sprite.trunc())
                        .as_uvec2();
                        // grab pixel
                        let pixel_index =
                            (texture_position.y * texture.width() + texture_position.x) as usize;
                        // check transparency
                        match texture.data.get(pixel_index * 4..(pixel_index * 4 + 4)) {
                            // If possible check the transparency bit is above cutoff
                            Some(pixel_data) if pixel_data[3] > settings.transparency_cutoff => {
                                true
                            }
                            // If not possible, it's not in the sprite
                            _ => false,
                        }
                    });

                blocked = cursor_in_valid_pixels_of_sprite
                    && picking_behavior
                        .map(|p| p.should_block_lower)
                        .unwrap_or(true);

                cursor_in_valid_pixels_of_sprite.then(|| {
                    let hit_pos_world =
                        sprite_transform.transform_point(cursor_pos_sprite.extend(0.0));
                    // Transform point from world to camera space to get the Z distance
                    let hit_pos_cam = cam_transform
                        .affine()
                        .inverse()
                        .transform_point3(hit_pos_world);
                    // HitData requires a depth as calculated from the camera's near clipping plane
                    let depth = -cam_ortho.near - hit_pos_cam.z;
                    (
                        entity,
                        HitData::new(
                            cam_entity,
                            depth,
                            Some(hit_pos_world),
                            Some(*sprite_transform.back()),
                        ),
                    )
                })
            })
            .collect();

        let order = camera.order as f32;
        output.send(PointerHits::new(*pointer, picks, order));
    }
}
