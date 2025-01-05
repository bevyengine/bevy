//! A [`bevy_picking`] backend for [`Text2d`]. Works for 2d text with arbitrary transforms.
//! Picking is done based on [`TextLayoutInfo`] bounds, not visible pixels. This means that
//! 2d text is pickable even in its transparent areas.
//!
//! **Note:** This backend is pretty much a 1:1 port of the [`bevy_sprite`] picking backend,
//! and *is not responsible for handling the picking of UI Text*. For that, please refer to the
//! picking backend implementation under the `bevy_ui` crate.

use core::cmp::Reverse;

use crate::{Text2d, TextLayoutInfo};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{prelude::*, FloatExt, FloatOrd};
use bevy_picking::backend::prelude::*;
use bevy_render::prelude::*;
use bevy_sprite::Anchor;
use bevy_transform::prelude::*;
use bevy_window::PrimaryWindow;

#[derive(Clone)]
pub struct Text2dPickingPlugin;

impl Plugin for Text2dPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, text2d_picking.in_set(PickSet::Backend));
    }
}

pub fn text2d_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &OrthographicProjection)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    text2d_query: Query<
        (
            Entity,
            &TextLayoutInfo,
            &Anchor,
            &GlobalTransform,
            Option<&PickingBehavior>,
            &ViewVisibility,
        ),
        With<Text2d>,
    >,
    mut output: EventWriter<PointerHits>,
) {
    let mut sorted_text2d: Vec<_> = text2d_query
        .iter()
        .filter_map(
            |(entity, text_layout_info, anchor, transform, picking_behavior, vis)| {
                if !transform.affine().is_nan() && vis.get() {
                    Some((
                        entity,
                        text_layout_info,
                        anchor,
                        transform,
                        picking_behavior,
                    ))
                } else {
                    None
                }
            },
        )
        .collect();

    sorted_text2d.sort_by_key(|x| Reverse(FloatOrd(x.3.translation().z)));

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

        let viewport_pos = camera
            .logical_viewport_rect()
            .map(|v| v.min)
            .unwrap_or_default();
        let pos_in_viewport = location.position - viewport_pos;

        let Ok(cursor_ray_world) = camera.viewport_to_world(cam_transform, pos_in_viewport) else {
            continue;
        };
        let cursor_ray_len = cam_ortho.far - cam_ortho.near;
        let cursor_ray_end = cursor_ray_world.origin + cursor_ray_world.direction * cursor_ray_len;

        let picks: Vec<(Entity, HitData)> = sorted_text2d
            .iter()
            .copied()
            .filter_map(
                |(entity, text_layout_info, anchor, text2d_transform, picking_behavior)| {
                    if blocked {
                        return None;
                    }

                    // Hit box in text2d coordinate system
                    let extents = text_layout_info.size;
                    let anchor = anchor.as_vec();
                    let center = -anchor * extents;
                    let rect = Rect::from_center_half_size(center, extents / 2.0);

                    // Transform cursor line segment to text2d coordinate system
                    let world_to_text2d = text2d_transform.affine().inverse();
                    let cursor_start_text2d =
                        world_to_text2d.transform_point3(cursor_ray_world.origin);
                    let cursor_end_text2d = world_to_text2d.transform_point3(cursor_ray_end);

                    // Find where the cursor segment intersects the plane Z=0 (which is the text2d's
                    // plane in text2d-local space). It may not intersect if, for example, we're
                    // viewing the text2d side-on
                    if cursor_start_text2d.z == cursor_end_text2d.z {
                        // Cursor ray is parallel to the text2d and misses it
                        return None;
                    }
                    let lerp_factor =
                        f32::inverse_lerp(cursor_start_text2d.z, cursor_end_text2d.z, 0.0);
                    if !(0.0..=1.0).contains(&lerp_factor) {
                        // Lerp factor is out of range, meaning that while an infinite line cast by
                        // the cursor would intersect the text2d, the text2d is not between the
                        // camera's near and far planes
                        return None;
                    }
                    // Otherwise we can interpolate the xy of the start and end positions by the
                    // lerp factor to get the cursor position in text2d space!
                    let cursor_pos_text2d = cursor_start_text2d
                        .lerp(cursor_end_text2d, lerp_factor)
                        .xy();

                    let is_cursor_in_text2d = rect.contains(cursor_pos_text2d);

                    blocked = is_cursor_in_text2d
                        && picking_behavior
                            .map(|p| p.should_block_lower)
                            .unwrap_or(true);

                    is_cursor_in_text2d.then(|| {
                        let hit_pos_world =
                            text2d_transform.transform_point(cursor_pos_text2d.extend(0.0));
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
                                Some(*text2d_transform.back()),
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
