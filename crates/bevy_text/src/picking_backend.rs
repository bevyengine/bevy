use bevy_app::{App, PreUpdate};
use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    observer::Trigger,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Query},
};
use bevy_math::{FloatExt, Rect, Vec2, Vec3Swizzles};
use bevy_picking::{
    backend::{HitData, PointerHits},
    events::{
        Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Move,
        Out, Over, Pointer, Pressed, Released,
    },
    pointer::{PointerId, PointerLocation},
    PickSet,
};
use bevy_reflect::Reflect;
use bevy_render::camera::{Camera, Projection};
use bevy_sprite::Anchor;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::PrimaryWindow;
use tracing::info;

use crate::{ComputedTextBlock, TextBounds, TextLayoutInfo};

pub(crate) fn plugin(app: &mut App) {
    app.add_observer(get_and_emit_text_hits::<Cancel>)
        .add_observer(get_and_emit_text_hits::<Click>)
        .add_observer(get_and_emit_text_hits::<Pressed>)
        .add_observer(get_and_emit_text_hits::<DragDrop>)
        .add_observer(get_and_emit_text_hits::<DragEnd>)
        .add_observer(get_and_emit_text_hits::<DragEnter>)
        .add_observer(get_and_emit_text_hits::<Drag>)
        .add_observer(get_and_emit_text_hits::<DragLeave>)
        .add_observer(get_and_emit_text_hits::<DragOver>)
        .add_observer(get_and_emit_text_hits::<DragStart>)
        .add_observer(get_and_emit_text_hits::<Move>)
        .add_observer(get_and_emit_text_hits::<Out>)
        .add_observer(get_and_emit_text_hits::<Over>)
        .add_observer(get_and_emit_text_hits::<Released>);

    app.add_systems(PreUpdate, text2d_picking.in_set(PickSet::Backend));
}

pub(crate) fn get_and_emit_text_hits<E: Clone + Reflect + std::fmt::Debug>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(&ComputedTextBlock, &TextLayoutInfo, &Anchor, &TextBounds)>,
    mut commands: Commands,
) {
    let Ok((c_text, text_layout, anchor, bounds)) = q.get(trigger.target) else {
        return;
    };
}

fn text2d_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &Projection)>,
    text_query: Query<(
        Entity,
        &TextLayoutInfo,
        &ComputedTextBlock,
        &Anchor,
        &TextBounds,
        &GlobalTransform,
    )>,
    mut output: EventWriter<PointerHits>,
) {
    let primary_window = primary_window.get_single().ok();

    for (pointer, location) in pointers.iter().filter_map(|(pointer, pointer_location)| {
        pointer_location.location().map(|loc| (pointer, loc))
    }) {
        // TODO: blocking
        let mut blocked = false;
        let Some((cam_entity, camera, cam_transform, Projection::Orthographic(cam_ortho))) =
            cameras
                .iter()
                .filter(|(_, camera, _, _)| {
                    // TODO: marker reqs
                    // let marker_requirement = !settings.require_markers || *cam_can_pick;
                    // camera.is_active && marker_requirement
                    true
                })
                .find(|(_, camera, _, _)| {
                    camera
                        .target
                        .normalize(primary_window)
                        .is_some_and(|x| x == location.target)
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

        // TODO: sort by Z
        let picks: Vec<(Entity, HitData)> = text_query
            .iter()
            .filter_map(|(entity, text_layout, b, c, text_bounds, text_transform)| {
                //
                // Transform cursor line segment to text coordinate system
                let world_to_text = text_transform.affine().inverse();
                let cursor_start_text = world_to_text.transform_point3(cursor_ray_world.origin);
                let cursor_end_text = world_to_text.transform_point3(cursor_ray_end);

                // Find where the cursor segment intersects the plane Z=0 (which is the sprite's
                // plane in sprite-local space). It may not intersect if, for example, we're
                // viewing the sprite side-on
                if cursor_start_text.z == cursor_end_text.z {
                    // Cursor ray is parallel to the sprite and misses it
                    return None;
                }
                let lerp_factor = f32::inverse_lerp(cursor_start_text.z, cursor_end_text.z, 0.0);
                if !(0.0..=1.0).contains(&lerp_factor) {
                    // Lerp factor is out of range, meaning that while an infinite line cast by
                    // the cursor would intersect the sprite, the sprite is not between the
                    // camera's near and far planes
                    return None;
                }

                // Otherwise we can interpolate the xy of the start and end positions by the
                // lerp factor to get the cursor position in sprite space!
                let relative_cursor_pos = cursor_start_text.lerp(cursor_end_text, lerp_factor).xy();

                // Find target rect, check cursor is contained inside
                let size = Vec2::new(
                    text_bounds.width.unwrap_or(text_layout.size.x),
                    text_bounds.height.unwrap_or(text_layout.size.y),
                );

                let text_rect = Rect::from_corners(-size / 2.0, size / 2.0);
                if !text_rect.contains(relative_cursor_pos) {
                    return None;
                }

                let hit_pos_world = text_transform.transform_point(relative_cursor_pos.extend(0.0));
                // Transform point from world to camera space to get the Z distance
                let hit_pos_cam = cam_transform
                    .affine()
                    .inverse()
                    .transform_point3(hit_pos_world);
                // HitData requires a depth as calculated from the camera's near clipping plane
                let depth = -cam_ortho.near - hit_pos_cam.z;

                Some((
                    entity,
                    HitData::new(
                        cam_entity,
                        depth,
                        Some(hit_pos_world),
                        Some(*text_transform.back()),
                    ),
                ))
            })
            .collect();

        let order = camera.order as f32;
        output.send(PointerHits::new(*pointer, picks, order));
    }
}
