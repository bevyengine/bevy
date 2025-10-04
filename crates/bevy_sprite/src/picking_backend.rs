//! A [`bevy_picking`] backend for sprites. Works for simple sprites and sprite atlases. Works for
//! sprites with arbitrary transforms.
//!
//! By default, picking for sprites is based on pixel opacity.
//! A sprite is picked only when a pointer is over an opaque pixel.
//! Alternatively, you can configure picking to be based on sprite bounds.
//!
//! ## Implementation Notes
//!
//! - The `position` reported in `HitData` in world space, and the `normal` is a normalized
//!   vector provided by the target's `GlobalTransform::back()`.

use crate::{Anchor, Sprite};
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_camera::{visibility::ViewVisibility, Camera, Projection};
use bevy_color::Alpha;
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_math::{prelude::*, FloatExt};
use bevy_picking::backend::prelude::*;
use bevy_reflect::prelude::*;
use bevy_transform::prelude::*;
use bevy_window::PrimaryWindow;

/// An optional component that marks cameras that should be used in the [`SpritePickingPlugin`].
///
/// Only needed if [`SpritePickingSettings::require_markers`] is set to `true`, and ignored
/// otherwise.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Debug, Default, Component, Clone)]
pub struct SpritePickingCamera;

/// How should the [`SpritePickingPlugin`] handle picking and how should it handle transparent pixels
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Debug, Clone)]
pub enum SpritePickingMode {
    /// Even if a sprite is picked on a transparent pixel, it should still count within the backend.
    /// Only consider the rect of a given sprite.
    BoundingBox,
    /// Ignore any part of a sprite which has a lower alpha value than the threshold (inclusive)
    /// Threshold is given as an f32 representing the alpha value in a Bevy Color Value
    AlphaThreshold(f32),
}

/// Runtime settings for the [`SpritePickingPlugin`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct SpritePickingSettings {
    /// When set to `true` sprite picking will only consider cameras marked with
    /// [`SpritePickingCamera`]. Defaults to `false`.
    /// Regardless of this setting, only sprites marked with [`Pickable`] will be considered.
    ///
    /// This setting is provided to give you fine-grained control over which cameras
    /// should be used by the sprite picking backend at runtime.
    pub require_markers: bool,
    /// Should the backend count transparent pixels as part of the sprite for picking purposes or should it use the bounding box of the sprite alone.
    ///
    /// Defaults to an inclusive alpha threshold of 0.1
    pub picking_mode: SpritePickingMode,
}

impl Default for SpritePickingSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
            picking_mode: SpritePickingMode::AlphaThreshold(0.1),
        }
    }
}

/// Enables the sprite picking backend, allowing you to click on, hover over and drag sprites.
#[derive(Clone)]
pub struct SpritePickingPlugin;

impl Plugin for SpritePickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpritePickingSettings>()
            .add_systems(PreUpdate, sprite_picking.in_set(PickingSystems::Backend));
    }
}

fn sprite_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
        &Projection,
        Has<SpritePickingCamera>,
    )>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    texture_atlas_layout: Res<Assets<TextureAtlasLayout>>,
    settings: Res<SpritePickingSettings>,
    sprite_query: Query<(
        Entity,
        &Sprite,
        &GlobalTransform,
        &Anchor,
        &Pickable,
        &ViewVisibility,
    )>,
    mut pointer_hits_writer: MessageWriter<PointerHits>,
) {
    let mut sorted_sprites: Vec<_> = sprite_query
        .iter()
        .filter_map(|(entity, sprite, transform, anchor, pickable, vis)| {
            if !transform.affine().is_nan() && vis.get() {
                Some((entity, sprite, transform, anchor, pickable))
            } else {
                None
            }
        })
        .collect();

    // radsort is a stable radix sort that performed better than `slice::sort_by_key`
    radsort::sort_by_key(&mut sorted_sprites, |(_, _, transform, _, _)| {
        -transform.translation().z
    });

    let primary_window = primary_window.single().ok();

    for (pointer, location) in pointers.iter().filter_map(|(pointer, pointer_location)| {
        pointer_location.location().map(|loc| (pointer, loc))
    }) {
        let mut blocked = false;
        let Some((cam_entity, camera, cam_transform, Projection::Orthographic(cam_ortho), _)) =
            cameras
                .iter()
                .filter(|(_, camera, _, _, cam_can_pick)| {
                    let marker_requirement = !settings.require_markers || *cam_can_pick;
                    camera.is_active && marker_requirement
                })
                .find(|(_, camera, _, _, _)| {
                    camera
                        .target
                        .normalize(primary_window)
                        .is_some_and(|x| x == location.target)
                })
        else {
            continue;
        };

        let viewport_pos = location.position;
        if let Some(viewport) = camera.logical_viewport_rect()
            && !viewport.contains(viewport_pos)
        {
            // The pointer is outside the viewport, skip it
            continue;
        }

        let Ok(cursor_ray_world) = camera.viewport_to_world(cam_transform, viewport_pos) else {
            continue;
        };
        let cursor_ray_len = cam_ortho.far - cam_ortho.near;
        let cursor_ray_end = cursor_ray_world.origin + cursor_ray_world.direction * cursor_ray_len;

        let picks: Vec<(Entity, HitData)> = sorted_sprites
            .iter()
            .copied()
            .filter_map(|(entity, sprite, sprite_transform, anchor, pickable)| {
                if blocked {
                    return None;
                }

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

                let Ok(cursor_pixel_space) = sprite.compute_pixel_space_point(
                    cursor_pos_sprite,
                    *anchor,
                    &images,
                    &texture_atlas_layout,
                ) else {
                    return None;
                };

                // Since the pixel space coordinate is `Ok`, we know the cursor is in the bounds of
                // the sprite.

                let cursor_in_valid_pixels_of_sprite = 'valid_pixel: {
                    match settings.picking_mode {
                        SpritePickingMode::AlphaThreshold(cutoff) => {
                            let Some(image) = images.get(&sprite.image) else {
                                // [`Sprite::from_color`] returns a defaulted handle.
                                // This handle doesn't return a valid image, so returning false here would make picking "color sprites" impossible
                                break 'valid_pixel true;
                            };
                            // grab pixel and check alpha
                            let Ok(color) = image.get_color_at(
                                cursor_pixel_space.x as u32,
                                cursor_pixel_space.y as u32,
                            ) else {
                                // We don't know how to interpret the pixel.
                                break 'valid_pixel false;
                            };
                            // Check the alpha is above the cutoff.
                            color.alpha() > cutoff
                        }
                        SpritePickingMode::BoundingBox => true,
                    }
                };

                blocked = cursor_in_valid_pixels_of_sprite && pickable.should_block_lower;

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
        pointer_hits_writer.write(PointerHits::new(*pointer, picks, order));
    }
}
