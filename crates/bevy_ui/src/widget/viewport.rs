use bevy_asset::Assets;
use bevy_ecs::{
    component::require,
    entity::Entity,
    event::EventReader,
    prelude::Component,
    query::{Changed, Or},
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_math::Rect;
#[cfg(feature = "bevy_ui_picking_backend")]
use bevy_picking::{
    events::PointerState,
    hover::HoverMap,
    pointer::{Location, PointerId, PointerInput},
    Pickable,
};
use bevy_platform_support::collections::HashSet;
use bevy_reflect::Reflect;
use bevy_render::{
    camera::{Camera, NormalizedRenderTarget},
    render_resource::Extent3d,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
#[cfg(feature = "bevy_ui_picking_backend")]
use uuid::Uuid;

use crate::{ComputedNode, Node};

/// Component used to render a [`Camera::target`]  to a node.
///
/// # See Also
///
/// [`on_add_viewport`]
/// [`update_viewport_render_target_size`]
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug)]
#[require(Node)]
#[cfg_attr(feature = "bevy_ui_picking_backend", require(Pickable (|| Pickable::IGNORE), PointerId (|| PointerId::Custom(Uuid::new_v4()))))]
pub struct ViewportNode {
    /// The entity representing the [`Camera`] associated with this viewport.
    ///
    /// Note that removing the [`ViewportNode`] component will not despawn this entity.
    pub camera: Entity,
}

impl ViewportNode {
    /// Creates a new [`ViewportNode`] with a given `camera`.
    pub fn new(camera: Entity) -> Self {
        Self { camera }
    }
}

#[cfg(feature = "bevy_ui_picking_backend")]
/// Handles viewport picking logic.
///
/// Viewport entities that are being hovered or dragged will have all pointer inputs sent to them.
pub fn viewport_picking(
    mut commands: Commands,
    viewport_query: Query<(&ViewportNode, &PointerId, &ComputedNode, &GlobalTransform)>,
    camera_query: Query<&Camera>,
    hover_map: Res<HoverMap>,
    pointer_state: Res<PointerState>,
    mut pointer_inputs: EventReader<PointerInput>,
    mut dragged_last_frame: Local<HashSet<(Entity, PointerId)>>,
) {
    let mut viewport_picks: HashSet<(Entity, PointerId)> = dragged_last_frame
        .drain()
        .chain(hover_map.iter().flat_map(|(hover_pointer_id, hits)| {
            hits.iter()
                .filter(|(entity, _)| viewport_query.contains(**entity))
                .map(|(entity, _)| (*entity, *hover_pointer_id))
        }))
        .collect();

    // Currently, we have only retrieved viewport entities if they are being hovered. However, this
    // does not allow dragging in-and-out of viewports.
    //
    // We resolve this by considering viewports that are being dragged.
    for ((pointer_id, _), pointer_state) in pointer_state.pointer_buttons.iter() {
        for &target in pointer_state
            .dragging
            .keys()
            .filter(|&entity| viewport_query.contains(*entity))
        {
            dragged_last_frame.insert((target, *pointer_id));
            viewport_picks.insert((target, *pointer_id));
        }
    }

    for (viewport_entity, pick_pointer_id) in viewport_picks {
        let Ok((&viewport, &viewport_pointer_id, computed_node, global_transform)) =
            viewport_query.get(viewport_entity)
        else {
            // This can only happen if entities in `dragged_last_frame` had one of these
            // components removed since we last queried them
            continue;
        };
        let Ok(camera) = camera_query.get(viewport.camera) else {
            continue;
        };
        let Some(cam_viewport_size) = camera.logical_viewport_size() else {
            continue;
        };

        // Create a `Rect` in *physical* coordinates centered at the node's GlobalTransform
        let node_rect = Rect::from_center_size(
            global_transform.translation().truncate(),
            computed_node.size(),
        );
        // Location::position uses *logical* coordinates
        let top_left = node_rect.min * computed_node.inverse_scale_factor();
        let logical_size = computed_node.size() * computed_node.inverse_scale_factor();

        let Some(target) = camera.target.as_image() else {
            continue;
        };

        for input in pointer_inputs
            .read()
            .filter(|input| input.pointer_id == pick_pointer_id)
        {
            let local_position = (input.location.position - top_left) / logical_size;
            let position = local_position * cam_viewport_size;

            let location = Location {
                position,
                target: NormalizedRenderTarget::Image(target.clone().into()),
            };

            commands.send_event(PointerInput {
                location,
                pointer_id: viewport_pointer_id,
                action: input.action,
            });
        }
    }
}

/// Updates the size of the associated render target for viewports when the node size changes.
pub fn update_viewport_render_target_size(
    viewport_query: Query<
        (&ViewportNode, &ComputedNode),
        Or<(Changed<ComputedNode>, Changed<ViewportNode>)>,
    >,
    camera_query: Query<&Camera>,
    mut images: ResMut<Assets<Image>>,
) {
    for (viewport, computed_node) in &viewport_query {
        let camera = camera_query.get(viewport.camera).unwrap();
        let size = computed_node.size();

        let Some(image_handle) = camera.target.as_image() else {
            continue;
        };
        let size = Extent3d {
            width: u32::max(1, size.x as u32),
            height: u32::max(1, size.y as u32),
            ..default()
        };
        images.get_mut(image_handle).unwrap().resize(size);
    }
}
