//! An automatic directional navigation system, powered by the [`AutoDirectionalNavigation`]
//! component.
//!
//! Unlike the navigation system provided by `bevy_input_focus`, the automatic directional
//! navigation system does not require specifying navigation edges. Just simply add the
//! [`AutoDirectionalNavigation`] component to your entities, and the system will automatically
//! calculate the navigation edges between entities based on screen position.
//!
//! [`AutoDirectionalNavigator`] replaces the manual directional navigation system
//! provided by the [`DirectionalNavigation`] system parameter from `bevy_input_focus`. The
//! [`AutoDirectionalNavigator`] will first navigate using manual override edges defined in the
//! [`DirectionalNavigationMap`](bevy_input_focus::directional_navigation::DirectionalNavigationMap).
//! If no manual overrides are defined, automatic navigation will occur between entities based on
//! screen position.
//!
//! If any resulting navigation behavior is undesired, [`AutoNavigationConfig`] can be tweaked or
//! manual overrides can be specified using the
//! [`DirectionalNavigationMap`](bevy_input_focus::directional_navigation::DirectionalNavigationMap).
//!
//! If nothing is focused when a navigation is requested (for example, because a stray mouse
//! click cleared the focus), [`AutoDirectionalNavigator::navigate`] picks a starting entity
//! instead of getting stuck. See its documentation for details.

use crate::{ComputedNode, ComputedUiTargetCamera, UiGlobalTransform};
use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::{ops, CompassOctant, Dir2, Vec2};

use bevy_input_focus::{
    directional_navigation::{
        AutoNavigationConfig, DirectionalNavigation, DirectionalNavigationError, FocusableArea,
    },
    navigator::find_best_candidate,
    AutoFocus, FocusCause, Focusable,
};

use bevy_reflect::{prelude::*, Reflect};

/// Marker component to enable automatic directional navigation to and from the entity.
///
/// Simply add this component to your UI entities so that the navigation algorithm will
/// consider this entity in its calculations:
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_ui::auto_directional_navigation::AutoDirectionalNavigation;
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
///    [`auto_generate_navigation_edges()`](bevy_input_focus::directional_navigation::auto_generate_navigation_edges)
///    separately for each layer:
///    ```rust,ignore
///    for layer in &layers {
///        let nodes: Vec<FocusableArea> = query_layer(layer).collect();
///        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);
///    }
///    ```
///
/// 2. **Manual cross-layer navigation**: Use
///    [`DirectionalNavigationMap::add_edge()`](bevy_input_focus::directional_navigation::DirectionalNavigationMap::add_edge)
///    to define explicit connections between layers (e.g., "Back" button to main menu).
///
/// 3. **Remove component when layer is hidden**: Dynamically add/remove
///    [`AutoDirectionalNavigation`] based on which layers are currently active.
///
/// See issue [#21679](https://github.com/bevyengine/bevy/issues/21679) for planned
/// improvements to layer-aware automatic navigation.
///
/// # Opting Out
///
/// To disable automatic navigation for specific entities:
///
/// - **Remove the component**: Simply don't add [`AutoDirectionalNavigation`] to entities
///   that should only use manual navigation edges.
/// - **Dynamically toggle**: Remove/insert the component at runtime to enable/disable
///   automatic navigation as needed.
///
/// Manual edges defined via [`DirectionalNavigationMap`](bevy_input_focus::directional_navigation::DirectionalNavigationMap)
/// will override any automatically calculated edges.
///
/// # Additional Requirements
///
/// Entities must also have:
/// - [`Focusable`] - automatically added as a required component
/// - [`ComputedNode`] - for size information
/// - [`UiGlobalTransform`] - for position information
///
/// These are automatically added by `bevy_ui` when you spawn UI entities.
///
/// # Custom UI Systems
///
/// For custom UI frameworks, you can call
/// [`auto_generate_navigation_edges`](bevy_input_focus::directional_navigation::auto_generate_navigation_edges)
/// directly in your own system instead of using this component.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[require(Focusable)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct AutoDirectionalNavigation {
    /// Whether to also consider `TabIndex` for navigation order hints.
    /// Currently unused but reserved for future functionality.
    pub respect_tab_order: bool,
}

/// A system parameter for combining manual and auto navigation between focusable entities in a directional way.
/// This wraps the [`DirectionalNavigation`] system parameter provided by `bevy_input_focus` and
/// augments it with auto directional navigation.
/// To use, the [`DirectionalNavigationPlugin`](bevy_input_focus::directional_navigation::DirectionalNavigationPlugin)
/// must be added to the app.
#[derive(SystemParam, Debug)]
pub struct AutoDirectionalNavigator<'w, 's> {
    /// A system parameter for the manual directional navigation system provided by `bevy_input_focus`
    pub manual_directional_navigation: DirectionalNavigation<'w, 's>,
    /// Configuration for the automated portion of the navigation algorithm.
    pub config: Res<'w, AutoNavigationConfig>,
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
            Has<AutoFocus>,
        ),
        (With<AutoDirectionalNavigation>, With<Focusable>),
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
    /// Returns the current input focus
    pub fn input_focus(&mut self) -> Option<Entity> {
        self.manual_directional_navigation.focus.get()
    }

    /// Tries to move the input focus in the given direction.
    ///
    /// If an entity is focused, manual edges from the
    /// [`DirectionalNavigationMap`](bevy_input_focus::directional_navigation::DirectionalNavigationMap)
    /// are followed first, then automatic navigation is used to find the best neighbor.
    ///
    /// If nothing is focused, a starting entity is picked from the visible entities with
    /// [`AutoDirectionalNavigation`], so that directional input can always recover focus:
    ///
    /// 1. If any of them have [`AutoFocus`], only those are considered.
    /// 2. The entity farthest in the requested direction is chosen.
    ///    For example, [`CompassOctant::North`] picks the top-most entity.
    /// 3. Ties are broken in reading order: top-most first, then left-most.
    ///
    /// Positions are compared within each entity's own render target, so with several
    /// UI cameras the choice may be made between entities on different targets.
    ///
    /// Returns the new focus if successful, and updates the [`InputFocus`](bevy_input_focus::InputFocus)
    /// resource to match.
    /// Returns [`DirectionalNavigationError::NoFocus`] if nothing is focused and there is no
    /// visible entity to focus, or another error if there is no neighbor in the requested direction.
    pub fn navigate(
        &mut self,
        direction: CompassOctant,
    ) -> Result<Entity, DirectionalNavigationError> {
        if let Some(current_focus) = self.input_focus() {
            // Respect manual edges first
            match self.manual_directional_navigation.navigate(direction) {
                Ok(new_focus) => {
                    self.manual_directional_navigation
                        .focus
                        .set(new_focus, FocusCause::Navigated);
                    Ok(new_focus)
                }
                Err(DirectionalNavigationError::NoNeighborInDirection { .. }) => {
                    if let Some((target_camera, origin)) =
                        self.entity_to_camera_and_focusable_area(current_focus)
                        && let Some(new_focus) = find_best_candidate(
                            &origin,
                            direction,
                            &self.get_navigable_nodes(target_camera),
                            &self.config,
                        )
                    {
                        self.manual_directional_navigation
                            .focus
                            .set(new_focus, FocusCause::Navigated);
                        Ok(new_focus)
                    } else {
                        Err(DirectionalNavigationError::NoNeighborInDirection {
                            current_focus,
                            direction,
                        })
                    }
                }
                err => err,
            }
        } else {
            self.focus_starting_entity(direction)
        }
    }

    /// Picks and focuses an entity when navigation is requested while nothing is focused.
    ///
    /// See [`Self::navigate`] for how the entity is chosen.
    fn focus_starting_entity(
        &mut self,
        direction: CompassOctant,
    ) -> Result<Entity, DirectionalNavigationError> {
        let mut autofocus_areas = Vec::new();
        let mut other_areas = Vec::new();
        for (
            entity,
            computed_target_camera,
            computed,
            transform,
            inherited_visibility,
            has_autofocus,
        ) in &self.navigable_entities_query
        {
            // Skip hidden or zero-size nodes, and nodes that are not rendered by any camera
            if computed.is_empty()
                || !inherited_visibility.get()
                || computed_target_camera.get().is_none()
            {
                continue;
            }
            let area = focusable_area(entity, computed, transform);
            if has_autofocus {
                autofocus_areas.push(area);
            } else {
                other_areas.push(area);
            }
        }

        let candidates = if autofocus_areas.is_empty() {
            &other_areas
        } else {
            &autofocus_areas
        };
        let new_focus = farthest_in_direction(candidates, direction)
            .ok_or(DirectionalNavigationError::NoFocus)?;
        self.manual_directional_navigation
            .focus
            .set(new_focus, FocusCause::Navigated);
        Ok(new_focus)
    }

    /// Returns a vec of [`FocusableArea`] representing nodes that are eligible to be automatically navigated to.
    /// The camera of any navigable nodes will equal the desired `target_camera`.
    fn get_navigable_nodes(&self, target_camera: Entity) -> Vec<FocusableArea> {
        self.navigable_entities_query
            .iter()
            .filter_map(
                |(entity, computed_target_camera, computed, transform, inherited_visibility, _)| {
                    // Skip hidden or zero-size nodes
                    if computed.is_empty() || !inherited_visibility.get() {
                        return None;
                    }
                    // Accept nodes that have the same target camera as the desired target camera
                    if let Some(tc) = computed_target_camera.get()
                        && tc == target_camera
                    {
                        Some(focusable_area(entity, computed, transform))
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
                computed_target_camera.get().map(|target_camera| {
                    (target_camera, focusable_area(entity, computed, transform))
                })
            },
        )
    }
}

/// Builds the [`FocusableArea`] of a UI node, in logical pixels.
fn focusable_area(
    entity: Entity,
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
) -> FocusableArea {
    let (scale, rotation, translation) = transform.to_scale_angle_translation();
    let scaled_size = computed.size() * computed.inverse_scale_factor() * scale;
    FocusableArea {
        entity,
        position: translation * computed.inverse_scale_factor(),
        size: get_rotated_bounds(scaled_size, rotation),
    }
}

/// Returns the entity whose center is farthest in `direction`.
///
/// Ties are broken in reading order: top-most first, then left-most, then by [`Entity`]
/// so that the result does not depend on query iteration order.
fn farthest_in_direction(areas: &[FocusableArea], direction: CompassOctant) -> Option<Entity> {
    // UI coordinates have Y+ pointing down, so flip the mathematical direction
    let dir = Dir2::from(direction).as_vec2() * Vec2::new(1.0, -1.0);
    areas
        .iter()
        .max_by(|a, b| {
            a.position
                .dot(dir)
                .total_cmp(&b.position.dot(dir))
                .then_with(|| b.position.y.total_cmp(&a.position.y))
                .then_with(|| b.position.x.total_cmp(&a.position.x))
                .then_with(|| b.entity.cmp(&a.entity))
        })
        .map(|area| area.entity)
}

/// Util used to get the resulting bounds of a UI entity after applying its rotation.
///
/// This is necessary to apply because navigation should only use the final screen position
/// of an entity in automatic navigation calculations. These bounds are used as the entity's size in
/// [`FocusableArea`].
fn get_rotated_bounds(size: Vec2, rotation: f32) -> Vec2 {
    if rotation == 0.0 {
        return size;
    }
    let cos_r = ops::cos(rotation).abs();
    let sin_r = ops::sin(rotation).abs();
    Vec2::new(
        size.x * cos_r + size.y * sin_r,
        size.x * sin_r + size.y * cos_r,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::App;
    use bevy_ecs::system::RunSystemOnce;
    use bevy_input_focus::{
        directional_navigation::DirectionalNavigationPlugin, AcquireFocus, InputFocus,
        InputFocusPlugin,
    };
    use bevy_window::Window;

    /// Creates an app with directional navigation set up, and returns it with a camera entity.
    fn setup_nav_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((InputFocusPlugin, DirectionalNavigationPlugin));
        let camera = app.world_mut().spawn_empty().id();
        (app, camera)
    }

    /// Spawns a visible 50x50 navigable node centered on `position`.
    fn spawn_nav_node(app: &mut App, camera: Entity, position: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                AutoDirectionalNavigation::default(),
                ComputedNode {
                    size: Vec2::splat(50.0),
                    ..Default::default()
                },
                UiGlobalTransform::from_translation(position),
                ComputedUiTargetCamera {
                    camera: Some(camera),
                },
                InheritedVisibility::VISIBLE,
            ))
            .id()
    }

    fn navigate(
        app: &mut App,
        direction: CompassOctant,
    ) -> Result<Entity, DirectionalNavigationError> {
        app.world_mut()
            .run_system_once(move |mut navigator: AutoDirectionalNavigator| {
                navigator.navigate(direction)
            })
            .unwrap()
    }

    fn clear_focus(app: &mut App) {
        app.world_mut().resource_mut::<InputFocus>().clear();
    }

    fn focus(app: &App) -> Option<Entity> {
        app.world().resource::<InputFocus>().get()
    }

    #[test]
    fn navigate_without_focus_picks_farthest_in_direction() {
        let (mut app, camera) = setup_nav_app();
        // A 2x2 grid
        let top_left = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 0.0));
        let top_right = spawn_nav_node(&mut app, camera, Vec2::new(100.0, 0.0));
        let bottom_left = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 100.0));
        let bottom_right = spawn_nav_node(&mut app, camera, Vec2::new(100.0, 100.0));

        for (direction, expected) in [
            // Ties are broken in reading order: top-most first, then left-most
            (CompassOctant::North, top_left),
            (CompassOctant::South, bottom_left),
            (CompassOctant::East, top_right),
            (CompassOctant::West, top_left),
            (CompassOctant::NorthEast, top_right),
            (CompassOctant::SouthEast, bottom_right),
            (CompassOctant::SouthWest, bottom_left),
            (CompassOctant::NorthWest, top_left),
        ] {
            clear_focus(&mut app);
            assert_eq!(navigate(&mut app, direction), Ok(expected), "{direction:?}");
            assert_eq!(focus(&app), Some(expected), "{direction:?}");
        }
    }

    #[test]
    fn navigate_without_focus_prefers_autofocus() {
        let (mut app, camera) = setup_nav_app();
        spawn_nav_node(&mut app, camera, Vec2::new(0.0, 0.0));
        let autofocus = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 100.0));
        spawn_nav_node(&mut app, camera, Vec2::new(0.0, 200.0));
        // Adding `AutoFocus` focuses the entity, so clear the focus afterwards
        app.world_mut().entity_mut(autofocus).insert(AutoFocus);
        clear_focus(&mut app);

        assert_eq!(navigate(&mut app, CompassOctant::North), Ok(autofocus));
        assert_eq!(focus(&app), Some(autofocus));
    }

    #[test]
    fn navigate_without_focus_skips_hidden_nodes() {
        let (mut app, camera) = setup_nav_app();
        let hidden_autofocus = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 100.0));
        app.world_mut()
            .entity_mut(hidden_autofocus)
            .insert((AutoFocus, InheritedVisibility::HIDDEN));
        let hidden_top = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 0.0));
        app.world_mut()
            .entity_mut(hidden_top)
            .insert(InheritedVisibility::HIDDEN);
        let visible = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 200.0));
        clear_focus(&mut app);

        assert_eq!(navigate(&mut app, CompassOctant::North), Ok(visible));
    }

    #[test]
    fn navigate_without_focus_or_candidates_returns_no_focus() {
        let (mut app, camera) = setup_nav_app();
        let hidden = spawn_nav_node(&mut app, camera, Vec2::ZERO);
        app.world_mut()
            .entity_mut(hidden)
            .insert(InheritedVisibility::HIDDEN);

        assert_eq!(
            navigate(&mut app, CompassOctant::North),
            Err(DirectionalNavigationError::NoFocus)
        );
        assert_eq!(focus(&app), None);
    }

    #[test]
    fn navigate_continues_normally_after_recovering_focus() {
        let (mut app, camera) = setup_nav_app();
        let top = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 0.0));
        let bottom = spawn_nav_node(&mut app, camera, Vec2::new(0.0, 100.0));

        assert_eq!(navigate(&mut app, CompassOctant::North), Ok(top));
        assert_eq!(navigate(&mut app, CompassOctant::South), Ok(bottom));
    }

    #[test]
    fn auto_directional_navigation_is_focusable_without_tab_index() {
        let mut app = App::new();
        app.add_plugins(InputFocusPlugin);
        let window = app.world_mut().spawn(Window::default()).id();
        let directional = app
            .world_mut()
            .spawn(AutoDirectionalNavigation::default())
            .id();

        assert!(app.world().entity(directional).contains::<Focusable>());
        app.world_mut().trigger(AcquireFocus {
            focused_entity: directional,
            window,
        });

        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(directional)
        );
    }
}
