mod convert;
pub mod debug;

use crate::{ContentSize, DefaultUiCamera, Measure, Node, Outline, Style, TargetCamera, UiScale};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::Entity,
    event::EventReader,
    query::{With, Without},
    removal_detection::RemovedComponents,
    system::{Query, Res, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_math::{UVec2, Vec2};
use bevy_render::camera::{Camera, NormalizedRenderTarget};
use bevy_transform::components::Transform;
use bevy_utils::tracing::warn;
use bevy_utils::{default, HashMap, HashSet};
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};
use std::fmt;
use taffy::TaffyTree;
use thiserror::Error;

pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f32, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
            min_size: physical_size.x.min(physical_size.y),
            max_size: physical_size.x.max(physical_size.y),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootNodePair {
    // The implicit "viewport" node created by Bevy
    implicit_viewport_node: taffy::tree::NodeId,
    // The root (parentless) node specified by the user
    user_root_node: taffy::tree::NodeId,
}

type UiTaffyTree = TaffyTree<Box<dyn Measure>>;

#[derive(Resource)]
pub struct UiSurface {
    entity_to_taffy: EntityHashMap<taffy::tree::NodeId>,
    camera_roots: EntityHashMap<Vec<RootNodePair>>,
    taffy: UiTaffyTree,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::tree::NodeId>>();
    _assert_send_sync::<UiTaffyTree>();
    _assert_send_sync::<UiSurface>();
}

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .field("camera_roots", &self.camera_roots)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.disable_rounding();
        Self {
            entity_to_taffy: Default::default(),
            camera_roots: Default::default(),
            taffy,
        }
    }
}

impl UiSurface {
    /// Retrieves the Taffy node associated with the given UI node entity and updates its style.
    /// If no associated Taffy node exists a new Taffy node is inserted into the Taffy layout.
    pub fn upsert_node(&mut self, entity: Entity, style: &Style, context: &LayoutContext) {
        let mut added = false;
        let taffy = &mut self.taffy;
        let taffy_node = self.entity_to_taffy.entry(entity).or_insert_with(|| {
            added = true;
            taffy.new_leaf(convert::from_style(context, style)).unwrap()
        });

        if !added {
            self.taffy
                .set_style(*taffy_node, convert::from_style(context, style))
                .unwrap();
        }
    }

    /// Update the [`Box<dyn Measure>`] context of the taffy node corresponding to the given [`Entity`] if the node exists.
    pub fn try_update_measure(&mut self, entity: Entity, measure: Box<dyn Measure>) -> Option<()> {
        let taffy_node = self.entity_to_taffy.get(&entity)?;

        self.taffy.set_node_context(*taffy_node, Some(measure)).ok()
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, entity: Entity, children: &Children) {
        let mut taffy_children = Vec::with_capacity(children.len());
        for child in children {
            if let Some(taffy_node) = self.entity_to_taffy.get(child) {
                taffy_children.push(*taffy_node);
            } else {
                warn!(
                    "Unstyled child in a UI entity hierarchy. You are using an entity \
without UI components as a child of an entity with UI components, results may be unexpected."
                );
            }
        }

        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy
            .set_children(*taffy_node, &taffy_children)
            .unwrap();
    }

    /// Removes children from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_children(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_children(*taffy_node, &[]).unwrap();
        }
    }

    /// Removes the measure from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_measure(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_node_context(*taffy_node, None).unwrap();
        }
    }

    /// Set the ui node entities without a [`Parent`] as children to the root node in the taffy layout.
    pub fn set_camera_children(
        &mut self,
        camera_id: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let viewport_style = taffy::style::Style {
            display: taffy::style::Display::Grid,
            // Note: Taffy percentages are floats ranging from 0.0 to 1.0.
            // So this is setting width:100% and height:100%
            size: taffy::geometry::Size {
                width: taffy::style::Dimension::Percent(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            align_items: Some(taffy::style::AlignItems::Start),
            justify_items: Some(taffy::style::JustifyItems::Start),
            ..default()
        };

        let existing_roots = self.camera_roots.entry(camera_id).or_default();
        let mut new_roots = Vec::new();
        for entity in children {
            let node = *self.entity_to_taffy.get(&entity).unwrap();
            let root_node = existing_roots
                .iter()
                .find(|n| n.user_root_node == node)
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(previous_parent) = self.taffy.parent(node) {
                        // remove the root node from the previous implicit node's children
                        self.taffy.remove_child(previous_parent, node).unwrap();
                    }

                    RootNodePair {
                        implicit_viewport_node: self
                            .taffy
                            .new_with_children(viewport_style.clone(), &[node])
                            .unwrap(),
                        user_root_node: node,
                    }
                });
            new_roots.push(root_node);
        }

        // Cleanup the implicit root nodes of any user root nodes that have been removed
        for old_root in existing_roots {
            if !new_roots.contains(old_root) {
                self.taffy.remove(old_root.implicit_viewport_node).unwrap();
            }
        }

        self.camera_roots.insert(camera_id, new_roots);
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout(&mut self, camera: Entity, render_target_resolution: UVec2) {
        let Some(camera_root_nodes) = self.camera_roots.get(&camera) else {
            return;
        };

        let available_space = taffy::geometry::Size {
            width: taffy::style::AvailableSpace::Definite(render_target_resolution.x as f32),
            height: taffy::style::AvailableSpace::Definite(render_target_resolution.y as f32),
        };
        for root_nodes in camera_root_nodes {
            self.taffy
                .compute_layout_with_measure(
                    root_nodes.implicit_viewport_node,
                    available_space,
                    |known_dimensions: taffy::geometry::Size<Option<f32>>,
                     available_space: taffy::geometry::Size<taffy::AvailableSpace>,
                     _node_id: taffy::tree::NodeId,
                     node_context: Option<&mut Box<dyn Measure>>| {
                        let size = node_context
                            .map(|measure| {
                                measure.measure(
                                    known_dimensions.width,
                                    known_dimensions.height,
                                    available_space.width,
                                    available_space.height,
                                )
                            })
                            .unwrap_or(Vec2::ZERO);

                        taffy::Size {
                            width: size.x,
                            height: size.y,
                        }
                    },
                )
                .unwrap();
        }
    }

    /// Removes each entity from the internal map and then removes their associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(node) = self.entity_to_taffy.remove(&entity) {
                self.taffy.remove(node).unwrap();
            }
        }
    }

    /// Get the layout geometry for the taffy node corresponding to the ui node [`Entity`].
    /// Does not compute the layout geometry, `compute_window_layouts` should be run before using this function.
    pub fn get_layout(&self, entity: Entity) -> Result<&taffy::Layout, LayoutError> {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy
                .layout(*taffy_node)
                .map_err(LayoutError::TaffyError)
        } else {
            warn!(
                "Styled child in a non-UI entity hierarchy. You are using an entity \
with UI components as a child of an entity without UI components, results may be unexpected."
            );
            Err(LayoutError::InvalidHierarchy)
        }
    }
}

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Invalid hierarchy")]
    InvalidHierarchy,
    #[error("Taffy error: {0}")]
    TaffyError(#[from] taffy::tree::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    cameras: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut resize_events: EventReader<bevy_window::WindowResized>,
    mut ui_surface: ResMut<UiSurface>,
    root_node_query: Query<(Entity, Option<&TargetCamera>), (With<Node>, Without<Parent>)>,
    style_query: Query<(Entity, Ref<Style>, Option<&TargetCamera>), With<Node>>,
    mut measure_query: Query<(Entity, &mut ContentSize)>,
    children_query: Query<(Entity, Ref<Children>), With<Node>>,
    just_children_query: Query<&Children>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_nodes: RemovedComponents<Node>,
    mut node_transform_query: Query<(&mut Node, &mut Transform)>,
) {
    struct CameraLayoutInfo {
        size: UVec2,
        resized: bool,
        scale_factor: f32,
        root_nodes: Vec<Entity>,
    }

    let camera_with_default = |target_camera: Option<&TargetCamera>| {
        target_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
    };

    let resized_windows: HashSet<Entity> = resize_events.read().map(|event| event.window).collect();
    let calculate_camera_layout_info = |camera: &Camera| {
        let size = camera.physical_viewport_size().unwrap_or(UVec2::ZERO);
        let scale_factor = camera.target_scaling_factor().unwrap_or(1.0);
        let camera_target = camera
            .target
            .normalize(primary_window.get_single().map(|(e, _)| e).ok());
        let resized = matches!(camera_target,
          Some(NormalizedRenderTarget::Window(window_ref)) if resized_windows.contains(&window_ref.entity())
        );
        CameraLayoutInfo {
            size,
            resized,
            scale_factor: scale_factor * ui_scale.0,
            root_nodes: Vec::new(),
        }
    };

    // Precalculate the layout info for each camera, so we have fast access to it for each node
    let mut camera_layout_info: HashMap<Entity, CameraLayoutInfo> = HashMap::new();
    for (entity, target_camera) in &root_node_query {
        match camera_with_default(target_camera) {
            Some(camera_entity) => {
                let Ok((_, camera)) = cameras.get(camera_entity) else {
                    warn!(
                        "TargetCamera (of root UI node {entity:?}) is pointing to a camera {:?} which doesn't exist",
                        camera_entity
                    );
                    continue;
                };
                let layout_info = camera_layout_info
                    .entry(camera_entity)
                    .or_insert_with(|| calculate_camera_layout_info(camera));
                layout_info.root_nodes.push(entity);
            }
            None => {
                if cameras.is_empty() {
                    warn!("No camera found to render UI to. To fix this, add at least one camera to the scene.");
                } else {
                    warn!(
                        "Multiple cameras found, causing UI target ambiguity. \
                        To fix this, add an explicit `TargetCamera` component to the root UI node {:?}",
                        entity
                    );
                }
                continue;
            }
        }
    }

    // Resize all nodes
    for (entity, style, target_camera) in style_query.iter() {
        if let Some(camera) =
            camera_with_default(target_camera).and_then(|c| camera_layout_info.get(&c))
        {
            if camera.resized
                || !scale_factor_events.is_empty()
                || ui_scale.is_changed()
                || style.is_changed()
            {
                let layout_context = LayoutContext::new(
                    camera.scale_factor,
                    [camera.size.x as f32, camera.size.y as f32].into(),
                );
                ui_surface.upsert_node(entity, &style, &layout_context);
            }
        }
    }
    scale_factor_events.clear();

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.read() {
        ui_surface.try_remove_measure(entity);
    }
    for (entity, mut content_size) in &mut measure_query {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.try_update_measure(entity, measure_func);
        }
    }

    // clean up removed nodes
    ui_surface.remove_entities(removed_nodes.read());

    // update camera children
    for (camera_id, CameraLayoutInfo { root_nodes, .. }) in &camera_layout_info {
        ui_surface.set_camera_children(*camera_id, root_nodes.iter().cloned());
    }

    // update and remove children
    for entity in removed_children.read() {
        ui_surface.try_remove_children(entity);
    }
    for (entity, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(entity, &children);
        }
    }

    for (camera_id, camera) in &camera_layout_info {
        let inverse_target_scale_factor = camera.scale_factor.recip();

        ui_surface.compute_camera_layout(*camera_id, camera.size);
        for root in &camera.root_nodes {
            update_uinode_geometry_recursive(
                *root,
                &ui_surface,
                &mut node_transform_query,
                &just_children_query,
                inverse_target_scale_factor,
                Vec2::ZERO,
                Vec2::ZERO,
            );
        }
    }

    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &UiSurface,
        node_transform_query: &mut Query<(&mut Node, &mut Transform)>,
        children_query: &Query<&Children>,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        mut absolute_location: Vec2,
    ) {
        if let Ok((mut node, mut transform)) = node_transform_query.get_mut(entity) {
            let Ok(layout) = ui_surface.get_layout(entity) else {
                return;
            };
            let layout_size =
                inverse_target_scale_factor * Vec2::new(layout.size.width, layout.size.height);
            let layout_location =
                inverse_target_scale_factor * Vec2::new(layout.location.x, layout.location.y);

            absolute_location += layout_location;

            let rounded_size = round_layout_coords(absolute_location + layout_size)
                - round_layout_coords(absolute_location);

            let rounded_location =
                round_layout_coords(layout_location) + 0.5 * (rounded_size - parent_size);

            // only trigger change detection when the new values are different
            if node.calculated_size != rounded_size || node.unrounded_size != layout_size {
                node.calculated_size = rounded_size;
                node.unrounded_size = layout_size;
            }
            if transform.translation.truncate() != rounded_location {
                transform.translation = rounded_location.extend(0.);
            }
            if let Ok(children) = children_query.get(entity) {
                for &child_uinode in children {
                    update_uinode_geometry_recursive(
                        child_uinode,
                        ui_surface,
                        node_transform_query,
                        children_query,
                        inverse_target_scale_factor,
                        rounded_size,
                        absolute_location,
                    );
                }
            }
        }
    }
}

/// Resolve and update the widths of Node outlines
pub fn resolve_outlines_system(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut outlines_query: Query<(&Outline, &mut Node)>,
) {
    let viewport_size = primary_window
        .get_single()
        .map(|window| window.size())
        .unwrap_or(Vec2::ZERO)
        / ui_scale.0;

    for (outline, mut node) in outlines_query.iter_mut() {
        let node = node.bypass_change_detection();
        node.outline_width = outline
            .width
            .resolve(node.size().x, viewport_size)
            .unwrap_or(0.)
            .max(0.);

        node.outline_offset = outline
            .offset
            .resolve(node.size().x, viewport_size)
            .unwrap_or(0.)
            .max(0.);
    }
}

#[inline]
/// Round `value` to the nearest whole integer, with ties (values with a fractional part equal to 0.5) rounded towards positive infinity.
fn round_ties_up(value: f32) -> f32 {
    if value.fract() != -0.5 {
        // The `round` function rounds ties away from zero. For positive numbers "away from zero" is towards positive infinity.
        // So for all positive values, and negative values with a fractional part not equal to 0.5, `round` returns the correct result.
        value.round()
    } else {
        // In the remaining cases, where `value` is negative and its fractional part is equal to 0.5, we use `ceil` to round it up towards positive infinity.
        value.ceil()
    }
}

#[inline]
/// Rounds layout coordinates by rounding ties upwards.
///
/// Rounding ties up avoids gaining a pixel when rounding bounds that span from negative to positive.
///
/// Example: The width between bounds of -50.5 and 49.5 before rounding is 100, using:
/// - `f32::round`: width becomes 101 (rounds to -51 and 50).
/// - `round_ties_up`: width is 100 (rounds to -50 and 50).
fn round_layout_coords(value: Vec2) -> Vec2 {
    Vec2 {
        x: round_ties_up(value.x),
        y: round_ties_up(value.y),
    }
}

#[cfg(test)]
mod tests {
    use crate::layout::round_layout_coords;
    use crate::prelude::*;
    use crate::ui_layout_system;
    use crate::update::update_target_camera_system;
    use crate::ContentSize;
    use crate::UiSurface;
    use bevy_asset::AssetEvent;
    use bevy_asset::Assets;
    use bevy_core_pipeline::core_2d::Camera2dBundle;
    use bevy_ecs::entity::Entity;
    use bevy_ecs::event::Events;
    use bevy_ecs::schedule::apply_deferred;
    use bevy_ecs::schedule::IntoSystemConfigs;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::world::World;
    use bevy_hierarchy::despawn_with_children_recursive;
    use bevy_hierarchy::BuildWorldChildren;
    use bevy_hierarchy::Children;
    use bevy_math::vec2;
    use bevy_math::Vec2;
    use bevy_render::camera::ManualTextureViews;
    use bevy_render::camera::OrthographicProjection;
    use bevy_render::texture::Image;
    use bevy_utils::prelude::default;
    use bevy_utils::HashMap;
    use bevy_window::PrimaryWindow;
    use bevy_window::Window;
    use bevy_window::WindowCreated;
    use bevy_window::WindowResized;
    use bevy_window::WindowResolution;
    use bevy_window::WindowScaleFactorChanged;
    use taffy::TraversePartialTree;

    #[test]
    fn round_layout_coords_must_round_ties_up() {
        assert_eq!(round_layout_coords(vec2(-50.5, 49.5)), vec2(-50., 50.));
    }

    // these window dimensions are easy to convert to and from percentage values
    const WINDOW_WIDTH: f32 = 1000.;
    const WINDOW_HEIGHT: f32 = 100.;

    fn setup_ui_test_world() -> (World, Schedule) {
        let mut world = World::new();
        world.init_resource::<UiScale>();
        world.init_resource::<UiSurface>();
        world.init_resource::<Events<WindowScaleFactorChanged>>();
        world.init_resource::<Events<WindowResized>>();
        // Required for the camera system
        world.init_resource::<Events<WindowCreated>>();
        world.init_resource::<Events<AssetEvent<Image>>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<ManualTextureViews>();

        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));
        world.spawn(Camera2dBundle::default());

        let mut ui_schedule = Schedule::default();
        ui_schedule.add_systems(
            (
                // UI is driven by calculated camera target info, so we need to run the camera system first
                bevy_render::camera::camera_system::<OrthographicProjection>,
                update_target_camera_system,
                apply_deferred,
                ui_layout_system,
            )
                .chain(),
        );

        (world, ui_schedule)
    }

    #[test]
    fn ui_nodes_with_percent_100_dimensions_should_fill_their_parent() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        // spawn a root entity with width and height set to fill 100% of its parent
        let ui_root = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        let ui_child = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        world.entity_mut(ui_root).add_child(ui_child);

        ui_schedule.run(&mut world);
        let ui_surface = world.resource::<UiSurface>();

        for ui_entity in [ui_root, ui_child] {
            let layout = ui_surface.get_layout(ui_entity).unwrap();
            assert_eq!(layout.size.width, WINDOW_WIDTH);
            assert_eq!(layout.size.height, WINDOW_HEIGHT);
        }
    }

    #[test]
    fn ui_surface_tracks_ui_entities() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        ui_schedule.run(&mut world);

        // no UI entities in world, none in UiSurface
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());

        let ui_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` should map `ui_entity` to a ui node in `UiSurface::entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert_eq!(ui_surface.entity_to_taffy.len(), 1);

        world.despawn(ui_entity);

        // `ui_layout_system` should remove `ui_entity` from `UiSurface::entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(!ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    #[test]
    #[should_panic]
    fn despawning_a_ui_entity_should_remove_its_corresponding_ui_node() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let ui_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        ui_schedule.run(&mut world);

        // retrieve the ui node corresponding to `ui_entity` from ui surface
        let ui_surface = world.resource::<UiSurface>();
        let ui_node = ui_surface.entity_to_taffy[&ui_entity];

        world.despawn(ui_entity);

        // `ui_layout_system` will receive a `RemovedComponents<Node>` event for `ui_entity`
        // and remove `ui_entity` from `ui_node` from the internal layout tree
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();

        // `ui_node` is removed, attempting to retrieve a style for `ui_node` panics
        let _ = ui_surface.taffy.style(ui_node);
    }

    #[test]
    fn changes_to_children_of_a_ui_entity_change_its_corresponding_ui_nodes_children() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let ui_parent_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        let ui_parent_node = ui_surface.entity_to_taffy[&ui_parent_entity];

        // `ui_parent_node` shouldn't have any children yet
        assert_eq!(ui_surface.taffy.child_count(ui_parent_node), 0);

        let mut ui_child_entities = (0..10)
            .map(|_| {
                let child = world.spawn(NodeBundle::default()).id();
                world.entity_mut(ui_parent_entity).add_child(child);
                child
            })
            .collect::<Vec<_>>();

        ui_schedule.run(&mut world);

        // `ui_parent_node` should have children now
        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node),
            ui_child_entities.len()
        );

        let child_node_map = HashMap::from_iter(
            ui_child_entities
                .iter()
                .map(|child_entity| (*child_entity, ui_surface.entity_to_taffy[child_entity])),
        );

        // the children should have a corresponding ui node and that ui node's parent should be `ui_parent_node`
        for node in child_node_map.values() {
            assert_eq!(ui_surface.taffy.parent(*node), Some(ui_parent_node));
        }

        // delete every second child
        let mut deleted_children = vec![];
        for i in (0..ui_child_entities.len()).rev().step_by(2) {
            let child = ui_child_entities.remove(i);
            world.despawn(child);
            deleted_children.push(child);
        }

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node),
            ui_child_entities.len()
        );

        // the remaining children should still have nodes in the layout tree
        for child_entity in &ui_child_entities {
            let child_node = child_node_map[child_entity];
            assert_eq!(ui_surface.entity_to_taffy[child_entity], child_node);
            assert_eq!(ui_surface.taffy.parent(child_node), Some(ui_parent_node));
            assert!(ui_surface
                .taffy
                .children(ui_parent_node)
                .unwrap()
                .contains(&child_node));
        }

        // the nodes of the deleted children should have been removed from the layout tree
        for deleted_child_entity in &deleted_children {
            assert!(!ui_surface
                .entity_to_taffy
                .contains_key(deleted_child_entity));
            let deleted_child_node = child_node_map[deleted_child_entity];
            assert!(!ui_surface
                .taffy
                .children(ui_parent_node)
                .unwrap()
                .contains(&deleted_child_node));
        }

        // despawn the parent entity and its descendants
        despawn_with_children_recursive(&mut world, ui_parent_entity);

        ui_schedule.run(&mut world);

        // all nodes should have been deleted
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    #[test]
    fn ui_node_should_be_set_to_its_content_size() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let content_size = Vec2::new(50., 25.);

        let ui_entity = world
            .spawn((
                NodeBundle {
                    style: Style {
                        align_self: AlignSelf::Start,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        let layout = ui_surface.get_layout(ui_entity).unwrap();

        // the node should takes its size from the fixed size measure func
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);
    }

    #[test]
    fn measure_funcs_should_be_removed_on_content_size_removal() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let content_size = Vec2::new(50., 25.);
        let ui_entity = world
            .spawn((
                NodeBundle {
                    style: Style {
                        align_self: AlignSelf::Start,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();

        let layout = ui_surface.get_layout(ui_entity).unwrap();
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        world.entity_mut(ui_entity).remove::<ContentSize>();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();

        // Without a content size, the node has no width or height constraints so the length of both dimensions is 0.
        let layout = ui_surface.get_layout(ui_entity).unwrap();
        assert_eq!(layout.size.width, 0.);
        assert_eq!(layout.size.height, 0.);
    }

    #[test]
    fn ui_rounding_test() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let parent = world
            .spawn(NodeBundle {
                style: Style {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::min_content(2),
                    margin: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|commands| {
                for _ in 0..2 {
                    commands.spawn(NodeBundle {
                        style: Style {
                            display: Display::Grid,
                            width: Val::Px(160.),
                            height: Val::Px(160.),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
            })
            .id();

        let children = world
            .entity(parent)
            .get::<Children>()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<Entity>>();

        for r in [2, 3, 5, 7, 11, 13, 17, 19, 21, 23, 29, 31].map(|n| (n as f32).recip()) {
            let mut s = r;
            while s <= 5. {
                world.resource_mut::<UiScale>().0 = s;
                ui_schedule.run(&mut world);
                let width_sum: f32 = children
                    .iter()
                    .map(|child| world.get::<Node>(*child).unwrap().calculated_size.x)
                    .sum();
                let parent_width = world.get::<Node>(parent).unwrap().calculated_size.x;
                assert!((width_sum - parent_width).abs() < 0.001);
                assert!((width_sum - 320.).abs() <= 1.);
                s += r;
            }
        }
    }
}
