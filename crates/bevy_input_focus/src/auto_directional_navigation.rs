//! An optional but recommended automatic directional navigation system, powered by
//! the [`AutoDirectionalNavigation`] component.
//! Prerequisites: Must have the `auto_nav` feature enabled.

use alloc::vec::Vec;
use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::CompassOctant;
use bevy_ui::{ComputedNode, ComputedUiTargetCamera, UiGlobalTransform};

use crate::navigator::{find_best_candidate, FocusableArea, NavigatorConfig};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};

/// Marker component to enable automatic directional navigation to and from the entity.
///
/// Simply add this component to your UI entities so that the navigation algorithm will
/// consider this entity in its calculations:
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_input_focus::directional_navigation::AutoDirectionalNavigation;
/// fn spawn_auto_nav_button(mut commands: Commands) {
///     commands.spawn((
///         // ... Button, Node, etc. ...
///         AutoDirectionalNavigation::default(), // That's it!
///     ));
/// }
/// ```
///
/// # Multi-Layer UIs and Z-Index
///
/// **Important**: Automatic navigation is currently **z-index agnostic** and treats
/// all entities with `AutoDirectionalNavigation` as a flat set, regardless of which UI layer
/// or z-index they belong to. This means navigation may jump between different layers (e.g.,
/// from a background menu to an overlay popup).
///
/// **Workarounds** for multi-layer UIs:
///
/// 1. **Per-layer manual edge generation**: Query entities by layer and call
///    [`auto_generate_navigation_edges()`](crate::directional_navigation::auto_generate_navigation_edges)
///    separately for each layer:
///    ```rust,ignore
///    for layer in &layers {
///        let nodes: Vec<FocusableArea> = query_layer(layer).collect();
///        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);
///    }
///    ```
///
/// 2. **Manual cross-layer navigation**: Use 
///   [`DirectionalNavigationMap::add_edge()`](crate::directional_navigation::DirectionalNavigationMap::add_edge)
///    to define explicit connections between layers (e.g., "Back" button to main menu).
///
/// 3. **Remove component when layer is hidden**: Dynamically add/remove
///    `AutoDirectionalNavigation` based on which layers are currently active.
///
/// See issue [#21679](https://github.com/bevyengine/bevy/issues/21679) for planned
/// improvements to layer-aware automatic navigation.
///
/// # Opting Out
///
/// To disable automatic navigation for specific entities:
///
/// - **Remove the component**: Simply don't add `AutoDirectionalNavigation` to entities
///   that should only use manual navigation edges.
/// - **Dynamically toggle**: Remove/insert the component at runtime to enable/disable
///   automatic navigation as needed.
///
/// Manual edges defined via [`DirectionalNavigationMap`](crate::directional_navigation::DirectionalNavigationMap) 
/// are completely independent and will continue to work regardless of this component.
///
/// # Requirements for `bevy_ui`
///
/// Entities must also have:
/// - [`ComputedNode`] - for size information
/// - [`UiGlobalTransform`] - for position information
///
/// These are automatically added by `bevy_ui` when you spawn UI entities.
///
/// # Custom UI Systems
///
/// For custom UI frameworks, you can call 
/// [`auto_generate_navigation_edges`](crate::directional_navigation::auto_generate_navigation_edges) 
/// directly in your own system instead of using this component.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, PartialEq, Clone)
)]
pub struct AutoDirectionalNavigation {
    /// Whether to also consider `TabIndex` for navigation order hints.
    /// Currently unused but reserved for future functionality.
    pub respect_tab_order: bool,
}

/// A system parameter for auto navigating between focusable entities in a directional way.
#[derive(SystemParam, Debug)]
pub(crate) struct AutoDirectionalNavigator<'w, 's> {
    /// Configuration for the automatic navigation system
    pub config: Res<'w, NavigatorConfig>,
    /// The entities which can possibly be navigated to automatically.
    navigable_entities_query: Query<
        'w,
        's,
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
        With<AutoDirectionalNavigation>,
    >,
    /// A query used to get the target camera and the [`FocusableArea`] for a given entity to be used in automatic navigation.
    camera_and_focusable_area_query: Query<
        'w,
        's,
        (
            Entity,
            &'static ComputedUiTargetCamera,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
        With<AutoDirectionalNavigation>,
    >,
}

impl<'w, 's> AutoDirectionalNavigator<'w, 's> {
    /// Tries to find the neighbor in a given direction from the given entity. Assumes the entity is valid.
    ///
    /// Returns a neighbor if successful.
    /// Returns None if there is no neighbor in the requested direction.
    pub fn get_neighbor(
        &mut self,
        from_entity: Entity,
        direction: CompassOctant,
    ) -> Option<Entity> {
        if let Some((target_camera, origin)) = self.entity_to_camera_and_focusable_area(from_entity)
            && let Some(new_focus) = find_best_candidate(
                &origin,
                direction,
                &self.get_navigable_nodes(target_camera),
                &self.config,
            )
        {
            Some(new_focus)
        } else {
            None
        }
    }

    /// Returns a vec of [`FocusableArea`] representing nodes that are eligible to be automatically navigated to.
    /// The camera of any navigable nodes will equal the desired `target_camera`.
    fn get_navigable_nodes(&self, target_camera: Entity) -> Vec<FocusableArea> {
        self.navigable_entities_query
            .iter()
            .filter_map(
                |(entity, computed_target_camera, computed, transform, inherited_visibility)| {
                    // Skip hidden or zero-size nodes
                    if computed.is_empty() || !inherited_visibility.get() {
                        return None;
                    }
                    // Accept nodes that have the same target camera as the desired target camera
                    if let Some(tc) = computed_target_camera.get()
                        && tc == target_camera
                    {
                        let (_scale, _rotation, translation) =
                            transform.to_scale_angle_translation();
                        Some(FocusableArea {
                            entity,
                            position: translation * computed.inverse_scale_factor(),
                            size: computed.size() * computed.inverse_scale_factor(),
                        })
                    } else {
                        // The node either does not have a target camera or it is not the same as the desired one.
                        None
                    }
                },
            )
            .collect()
    }

    /// Gets the target camera and the [`FocusableArea`] of the provided entity, if it exists.
    ///
    /// Returns None if there was a [`QueryEntityError`](bevy_ecs::query::QueryEntityError) or
    /// if the entity does not have a target camera.
    fn entity_to_camera_and_focusable_area(
        &self,
        entity: Entity,
    ) -> Option<(Entity, FocusableArea)> {
        self.camera_and_focusable_area_query.get(entity).map_or(
            None,
            |(entity, computed_target_camera, computed, transform)| {
                if let Some(target_camera) = computed_target_camera.get() {
                    let (_scale, _rotation, translation) = transform.to_scale_angle_translation();
                    Some((
                        target_camera,
                        FocusableArea {
                            entity,
                            position: translation * computed.inverse_scale_factor(),
                            size: computed.size() * computed.inverse_scale_factor(),
                        },
                    ))
                } else {
                    None
                }
            },
        )
    }
}
