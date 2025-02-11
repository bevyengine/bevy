use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectResource;
use bevy_ecs::query::Has;
use bevy_ecs::system::Res;
use bevy_ecs::{
    entity::Entity, event::EventWriter, query::With, resource::Resource,
    schedule::IntoSystemConfigs, system::Query,
};
use bevy_math::{FloatExt, Rect, Vec2, Vec3Swizzles};
use bevy_picking::Pickable;
use bevy_picking::{
    backend::{HitData, PointerHits},
    pointer::{PointerId, PointerLocation},
    PickSet,
};
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::camera::{Camera, Projection};
use bevy_render::view::ViewVisibility;
use bevy_transform::components::GlobalTransform;
use bevy_window::PrimaryWindow;

use crate::{Text2d, TextBounds, TextLayoutInfo};

/// Runtime settings for the [`Text2dPickingPlugin`].
#[derive(Default, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct Text2dPickingSettings {
    /// When set to `true` picking will only consider cameras marked with
    /// [`Text2dPickingCamera`] and entities marked with [`Pickable`]. `false` by default.
    ///
    /// This setting is provided to give you fine-grained control over which cameras and entities
    /// should be used by the picking backend at runtime.
    pub require_markers: bool,
}

pub struct Text2dPickingPlugin;

impl Plugin for Text2dPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Text2dPickingSettings>()
            .register_type::<Text2dPickingSettings>()
            .add_systems(PreUpdate, text2d_picking.in_set(PickSet::Backend));
    }
}

#[derive(Component)]
pub struct Text2dPickingCamera;

fn text2d_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
        &Projection,
        Has<Text2dPickingCamera>,
    )>,
    text_query: Query<
        (
            Entity,
            &TextLayoutInfo,
            &TextBounds,
            &GlobalTransform,
            Option<&Pickable>,
            &ViewVisibility,
        ),
        With<Text2d>,
    >,
    settings: Res<Text2dPickingSettings>,
    mut output: EventWriter<PointerHits>,
) {
    let mut sorted_texts: Vec<_> = text_query
        .iter()
        .filter_map(|(entity, layout, bounds, transform, pickable, vis)| {
            let marker_requirement = !settings.require_markers || pickable.is_some();
            if !transform.affine().is_nan() && vis.get() && marker_requirement {
                Some((entity, layout, bounds, transform, pickable))
            } else {
                None
            }
        })
        .collect();

    radsort::sort_by_key(&mut sorted_texts, |(_, _, _, t, _)| -t.translation().z);

    let primary_window = primary_window.get_single().ok();

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

        let picks: Vec<(Entity, HitData)> = sorted_texts
            .iter()
            .filter_map(
                |(entity, text_layout, text_bounds, text_transform, pickable)| {
                    if blocked {
                        return None;
                    }
                    //
                    // Transform cursor line segment to text coordinate system
                    let world_to_text = text_transform.affine().inverse();
                    let cursor_start_text = world_to_text.transform_point3(cursor_ray_world.origin);
                    let cursor_end_text = world_to_text.transform_point3(cursor_ray_end);

                    // Find where the cursor segment intersects the plane Z=0 (which is the text's
                    // plane in local space). It may not intersect if, for example, we're
                    // viewing the text side-on
                    if cursor_start_text.z == cursor_end_text.z {
                        // Cursor ray is parallel to the text and misses it
                        return None;
                    }
                    let lerp_factor =
                        f32::inverse_lerp(cursor_start_text.z, cursor_end_text.z, 0.0);
                    if !(0.0..=1.0).contains(&lerp_factor) {
                        // Lerp factor is out of range, meaning that while an infinite line cast by
                        // the cursor would intersect the text, the text is not between the
                        // camera's near and far planes
                        return None;
                    }

                    // Otherwise we can interpolate the xy of the start and end positions by the
                    // lerp factor to get the cursor position in local space
                    let relative_cursor_pos =
                        cursor_start_text.lerp(cursor_end_text, lerp_factor).xy();

                    // Find target rect, check cursor is contained inside
                    let size = Vec2::new(
                        text_bounds.width.unwrap_or(text_layout.size.x),
                        text_bounds.height.unwrap_or(text_layout.size.y),
                    );

                    let text_rect = Rect::from_corners(-size / 2.0, size / 2.0);

                    if !text_rect.contains(relative_cursor_pos) {
                        return None;
                    }

                    blocked = pickable.is_none_or(|p| p.should_block_lower);

                    let hit_pos_world =
                        text_transform.transform_point(relative_cursor_pos.extend(0.0));
                    // Transform point from world to camera space to get the Z distance
                    let hit_pos_cam = cam_transform
                        .affine()
                        .inverse()
                        .transform_point3(hit_pos_world);
                    // HitData requires a depth as calculated from the camera's near clipping plane
                    let depth = -cam_ortho.near - hit_pos_cam.z;

                    Some((
                        *entity,
                        HitData::new(
                            cam_entity,
                            depth,
                            Some(hit_pos_world),
                            Some(*text_transform.back()),
                        ),
                    ))
                },
            )
            .collect();

        let order = camera.order as f32;
        output.send(PointerHits::new(*pointer, picks, order));
    }
}
