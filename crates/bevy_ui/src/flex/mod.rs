mod convert;

use crate::{CalculatedSize, Node, Style, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    event::EventReader,
    query::{Changed, ReadOnlyWorldQuery, With, Without},
    removal_detection::RemovedComponents,
    system::{Query, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window, WindowResolution, WindowScaleFactorChanged};
use std::fmt;
use taffy::{
    prelude::{AvailableSpace, Size},
    style_helpers::TaffyMaxContent,
    Taffy,
};

pub struct LayoutContext {
    pub scale_factor: f64,
    pub physical_size: Vec2,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f64, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
            min_size: physical_size.x.min(physical_size.y),
            max_size: physical_size.x.max(physical_size.y),
        }
    }
}

#[derive(Resource)]
pub struct FlexSurface {
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    window_nodes: HashMap<Entity, taffy::node::Node>,
    taffy: Taffy,
}

// SAFETY: as long as MeasureFunc is Send + Sync. https://github.com/DioxusLabs/taffy/issues/146
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

fn _assert_send_sync_flex_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, taffy::node::Node>>();
    _assert_send_sync::<Taffy>();
}

impl fmt::Debug for FlexSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FlexSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .field("window_nodes", &self.window_nodes)
            .finish()
    }
}

impl Default for FlexSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            window_nodes: Default::default(),
            taffy: Taffy::new(),
        }
    }
}

impl FlexSurface {
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

    pub fn upsert_leaf(
        &mut self,
        entity: Entity,
        style: &Style,
        calculated_size: CalculatedSize,
        context: &LayoutContext,
    ) {
        let taffy = &mut self.taffy;
        let taffy_style = convert::from_style(context, style);
        let scale_factor = context.scale_factor;
        let measure = taffy::node::MeasureFunc::Boxed(Box::new(
            move |constraints: Size<Option<f32>>, _available: Size<AvailableSpace>| {
                let mut size = Size {
                    width: (scale_factor * calculated_size.size.x as f64) as f32,
                    height: (scale_factor * calculated_size.size.y as f64) as f32,
                };
                match (constraints.width, constraints.height) {
                    (None, None) => {}
                    (Some(width), None) => {
                        if calculated_size.preserve_aspect_ratio {
                            size.height = width * size.height / size.width;
                        }
                        size.width = width;
                    }
                    (None, Some(height)) => {
                        if calculated_size.preserve_aspect_ratio {
                            size.width = height * size.width / size.height;
                        }
                        size.height = height;
                    }
                    (Some(width), Some(height)) => {
                        size.width = width;
                        size.height = height;
                    }
                }
                size
            },
        ));
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_style(*taffy_node, taffy_style).unwrap();
            self.taffy.set_measure(*taffy_node, Some(measure)).unwrap();
        } else {
            let taffy_node = taffy.new_leaf_with_measure(taffy_style, measure).unwrap();
            self.entity_to_taffy.insert(entity, taffy_node);
        }
    }

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
            self.taffy.set_measure(*taffy_node, None).unwrap();
        }
    }

    pub fn update_window(&mut self, window: Entity, window_resolution: &WindowResolution) {
        let taffy = &mut self.taffy;
        let node = self
            .window_nodes
            .entry(window)
            .or_insert_with(|| taffy.new_leaf(taffy::style::Style::default()).unwrap());

        taffy
            .set_style(
                *node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(
                            window_resolution.physical_width() as f32
                        ),
                        height: taffy::style::Dimension::Points(
                            window_resolution.physical_height() as f32,
                        ),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    pub fn set_window_children(
        &mut self,
        parent_window: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let taffy_node = self.window_nodes.get(&parent_window).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_taffy.get(&e).unwrap())
            .collect::<Vec<taffy::node::Node>>();
        self.taffy.set_children(*taffy_node, &child_nodes).unwrap();
    }

    pub fn compute_window_layouts(&mut self) {
        for window_node in self.window_nodes.values() {
            self.taffy
                .compute_layout(*window_node, Size::MAX_CONTENT)
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

    pub fn get_layout(&self, entity: Entity) -> Result<&taffy::layout::Layout, FlexError> {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy
                .layout(*taffy_node)
                .map_err(FlexError::TaffyError)
        } else {
            warn!(
                "Styled child in a non-UI entity hierarchy. You are using an entity \
with UI components as a child of an entity without UI components, results may be unexpected."
            );
            Err(FlexError::InvalidHierarchy)
        }
    }
}

#[derive(Debug)]
pub enum FlexError {
    InvalidHierarchy,
    TaffyError(taffy::error::TaffyError),
}

#[allow(clippy::too_many_arguments)]
pub fn flex_node_system(
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut resize_events: EventReader<bevy_window::WindowResized>,
    mut flex_surface: ResMut<FlexSurface>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    node_query: Query<(Entity, &Style, Option<&CalculatedSize>), (With<Node>, Changed<Style>)>,
    full_node_query: Query<(Entity, &Style, Option<&CalculatedSize>), With<Node>>,
    changed_size_query: Query<
        (Entity, &Style, &CalculatedSize),
        (With<Node>, Changed<CalculatedSize>),
    >,
    children_query: Query<(Entity, &Children), (With<Node>, Changed<Children>)>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_calculated_sizes: RemovedComponents<CalculatedSize>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
    mut removed_nodes: RemovedComponents<Node>,
) {
    // assume one window for time being...
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let (primary_window_entity, logical_to_physical_factor, physical_size) =
        if let Ok((entity, primary_window)) = primary_window.get_single() {
            (
                entity,
                primary_window.resolution.scale_factor(),
                Vec2::new(
                    primary_window.resolution.physical_width() as f32,
                    primary_window.resolution.physical_height() as f32,
                ),
            )
        } else {
            return;
        };

    let resized = resize_events
        .iter()
        .any(|resized_window| resized_window.window == primary_window_entity);

    // update window root nodes
    for (entity, window) in windows.iter() {
        flex_surface.update_window(entity, &window.resolution);
    }

    let scale_factor = logical_to_physical_factor * ui_scale.scale;

    let viewport_values = LayoutContext::new(scale_factor, physical_size);

    fn update_changed<F: ReadOnlyWorldQuery>(
        flex_surface: &mut FlexSurface,
        viewport_values: &LayoutContext,
        query: Query<(Entity, &Style, Option<&CalculatedSize>), F>,
    ) {
        // update changed nodes
        for (entity, style, calculated_size) in &query {
            // TODO: remove node from old hierarchy if its root has changed
            if let Some(calculated_size) = calculated_size {
                flex_surface.upsert_leaf(entity, style, *calculated_size, viewport_values);
            } else {
                flex_surface.upsert_node(entity, style, viewport_values);
            }
        }
    }

    if !scale_factor_events.is_empty() || ui_scale.is_changed() || resized {
        scale_factor_events.clear();
        update_changed(&mut flex_surface, &viewport_values, full_node_query);
    } else {
        update_changed(&mut flex_surface, &viewport_values, node_query);
    }

    for (entity, style, calculated_size) in &changed_size_query {
        flex_surface.upsert_leaf(entity, style, *calculated_size, &viewport_values);
    }

    // clean up removed nodes
    flex_surface.remove_entities(removed_nodes.iter());

    // When a `CalculatedSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_calculated_sizes.iter() {
        flex_surface.try_remove_measure(entity);
    }

    // update window children (for now assuming all Nodes live in the primary window)
    flex_surface.set_window_children(primary_window_entity, root_node_query.iter());

    // update and remove children
    for entity in removed_children.iter() {
        flex_surface.try_remove_children(entity);
    }
    for (entity, children) in &children_query {
        flex_surface.update_children(entity, children);
    }

    // compute layouts
    flex_surface.compute_window_layouts();

    let physical_to_logical_factor = 1. / logical_to_physical_factor;

    let to_logical = |v| (physical_to_logical_factor * v as f64) as f32;

    // PERF: try doing this incrementally
    for (entity, mut node, mut transform, parent) in &mut node_transform_query {
        let layout = flex_surface.get_layout(entity).unwrap();
        let new_size = Vec2::new(
            to_logical(layout.size.width),
            to_logical(layout.size.height),
        );
        // only trigger change detection when the new value is different
        if node.calculated_size != new_size {
            node.calculated_size = new_size;
        }
        let mut new_position = transform.translation;
        new_position.x = to_logical(layout.location.x + layout.size.width / 2.0);
        new_position.y = to_logical(layout.location.y + layout.size.height / 2.0);
        if let Some(parent) = parent {
            if let Ok(parent_layout) = flex_surface.get_layout(**parent) {
                new_position.x -= to_logical(parent_layout.size.width / 2.0);
                new_position.y -= to_logical(parent_layout.size.height / 2.0);
            }
        }
        // only trigger change detection when the new value is different
        if transform.translation != new_position {
            transform.translation = new_position;
        }
    }
}
