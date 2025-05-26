use bevy_asset::Assets;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    query::{Changed, Or},
    reflect::ReflectComponent,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_math::Rect;
#[cfg(feature = "bevy_ui_picking_backend")]
use bevy_picking::{
    events::PointerState,
    hover::HoverMap,
    pointer::{Location, PointerId, PointerInput, PointerLocation},
};
use bevy_platform::collections::HashMap;
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
/// [`update_viewport_render_target_size`]
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug)]
#[require(Node)]
#[cfg_attr(
    feature = "bevy_ui_picking_backend",
    require(PointerId::Custom(Uuid::new_v4()))
)]
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
    mut viewport_query: Query<(
        Entity,
        &ViewportNode,
        &PointerId,
        &mut PointerLocation,
        &ComputedNode,
        &GlobalTransform,
    )>,
    camera_query: Query<&Camera>,
    hover_map: Res<HoverMap>,
    pointer_state: Res<PointerState>,
    mut pointer_inputs: EventReader<PointerInput>,
) {
    // Handle hovered entities.
    let mut viewport_picks: HashMap<Entity, PointerId> = hover_map
        .iter()
        .flat_map(|(hover_pointer_id, hits)| {
            hits.iter()
                .filter(|(entity, _)| viewport_query.contains(**entity))
                .map(|(entity, _)| (*entity, *hover_pointer_id))
        })
        .collect();

    // Handle dragged entities, which need to be considered for dragging in and out of viewports.
    for ((pointer_id, _), pointer_state) in pointer_state.pointer_buttons.iter() {
        for &target in pointer_state
            .dragging
            .keys()
            .filter(|&entity| viewport_query.contains(*entity))
        {
            viewport_picks.insert(target, *pointer_id);
        }
    }

    for (
        viewport_entity,
        &viewport,
        &viewport_pointer_id,
        mut viewport_pointer_location,
        computed_node,
        global_transform,
    ) in &mut viewport_query
    {
        let Some(pick_pointer_id) = viewport_picks.get(&viewport_entity) else {
            // Lift the viewport pointer if it's not being used.
            viewport_pointer_location.location = None;
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
            .filter(|input| &input.pointer_id == pick_pointer_id)
        {
            let local_position = (input.location.position - top_left) / logical_size;
            let position = local_position * cam_viewport_size;

            let location = Location {
                position,
                target: NormalizedRenderTarget::Image(target.clone().into()),
            };
            viewport_pointer_location.location = Some(location.clone());

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
        let image = images.get_mut(image_handle).unwrap();
        if image.data.is_some() {
            image.resize(size);
        } else {
            image.texture_descriptor.size = size;
        }
    }
}
