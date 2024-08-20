//! A [`bevy_picking`] backend for sprites. Works for simple sprites and sprite atlases. Works for
//! sprites with arbitrary transforms. Picking is done based on sprite bounds, not visible pixels.
//! This means a partially transparent sprite is pickable even in its transparent areas.

use std::cmp::Ordering;

use crate::{Sprite, TextureAtlas, TextureAtlasLayout};
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{prelude::*, FloatExt};
use bevy_picking::backend::prelude::*;
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use bevy_window::PrimaryWindow;

#[derive(Clone)]
pub struct SpritePickingBackend;

impl Plugin for SpritePickingBackend {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, sprite_picking.in_set(PickSet::Backend));
    }
}

pub fn sprite_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &OrthographicProjection)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    texture_atlas_layout: Res<Assets<TextureAtlasLayout>>,
    sprite_query: Query<
        (
            Entity,
            Option<&Sprite>,
            Option<&TextureAtlas>,
            Option<&Handle<Image>>,
            &GlobalTransform,
            Option<&Pickable>,
            &ViewVisibility,
        ),
        Or<(With<Sprite>, With<TextureAtlas>)>,
    >,
    mut output: EventWriter<PointerHits>,
) {
    let mut sorted_sprites: Vec<_> = sprite_query.iter().collect();
    sorted_sprites.sort_by(|a, b| {
        (b.4.translation().z)
            .partial_cmp(&a.4.translation().z)
            .unwrap_or(Ordering::Equal)
    });

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

        let Some(cursor_ray_world) = camera.viewport_to_world(cam_transform, location.position)
        else {
            continue;
        };
        let cursor_ray_len = cam_ortho.far - cam_ortho.near;
        let cursor_ray_end = cursor_ray_world.origin + cursor_ray_world.direction * cursor_ray_len;

        let picks: Vec<(Entity, HitData)> = sorted_sprites
            .iter()
            .copied()
            .filter(|(.., visibility)| visibility.get())
            .filter_map(
                |(entity, sprite, atlas, image, sprite_transform, pickable, ..)| {
                    if blocked {
                        return None;
                    }

                    // Hit box in sprite coordinate system
                    let (extents, anchor) = if let Some((sprite, atlas)) = sprite.zip(atlas) {
                        let extents = sprite.custom_size.or_else(|| {
                            texture_atlas_layout
                                .get(&atlas.layout)
                                .map(|f| f.textures[atlas.index].size().as_vec2())
                        })?;
                        let anchor = sprite.anchor.as_vec();
                        (extents, anchor)
                    } else if let Some((sprite, image)) = sprite.zip(image) {
                        let extents = sprite
                            .custom_size
                            .or_else(|| images.get(image).map(|f| f.size().as_vec2()))?;
                        let anchor = sprite.anchor.as_vec();
                        (extents, anchor)
                    } else {
                        return None;
                    };

                    let center = -anchor * extents;
                    let rect = Rect::from_center_half_size(center, extents / 2.0);

                    // Transform cursor line segment to sprite coordinate system
                    let world_to_sprite = sprite_transform.affine().inverse();
                    let cursor_start_sprite =
                        world_to_sprite.transform_point3(cursor_ray_world.origin);
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

                    blocked = is_cursor_in_sprite
                        && pickable.map(|p| p.should_block_lower) != Some(false);

                    is_cursor_in_sprite.then(|| {
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
                },
            )
            .collect();

        let order = camera.order as f32;
        output.send(PointerHits::new(*pointer, picks, order));
    }
}
