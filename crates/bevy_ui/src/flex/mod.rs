mod convert;

use crate::{CalculatedSize, Node, Style, UiScale};
use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    query::{Changed, With, Without, WorldQuery},
    system::{Query, RemovedComponents, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{Window, WindowId, WindowScaleFactorChanged, Windows};
use std::fmt;
use taffy::{number::Number, Taffy};

#[derive(Resource)]
pub struct FlexSurface {
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    window_nodes: HashMap<WindowId, taffy::node::Node>,
    taffy: Taffy,
}

// SAFETY: as long as MeasureFunc is Send + Sync. https://github.com/DioxusLabs/taffy/issues/146
// TODO: remove allow on lint - https://github.com/bevyengine/bevy/issues/3666
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

fn _assert_send_sync_flex_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, taffy::node::Node>>();
    _assert_send_sync::<HashMap<WindowId, taffy::node::Node>>();
    // FIXME https://github.com/DioxusLabs/taffy/issues/146
    // _assert_send_sync::<Taffy>();
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
    pub fn upsert_node(&mut self, entity: Entity, style: &Style, scale_factor: f64) {
        let mut added = false;
        let taffy = &mut self.taffy;
        let taffy_style = convert::from_style(scale_factor, style);
        let taffy_node = self.entity_to_taffy.entry(entity).or_insert_with(|| {
            added = true;
            taffy.new_node(taffy_style, &Vec::new()).unwrap()
        });

        if !added {
            self.taffy.set_style(*taffy_node, taffy_style).unwrap();
        }
    }

    pub fn upsert_leaf(
        &mut self,
        entity: Entity,
        style: &Style,
        calculated_size: CalculatedSize,
        scale_factor: f64,
    ) {
        let taffy = &mut self.taffy;
        let taffy_style = convert::from_style(scale_factor, style);
        let measure = taffy::node::MeasureFunc::Boxed(Box::new(
            move |constraints: taffy::geometry::Size<Number>| {
                let mut size = convert::from_f32_size(scale_factor, calculated_size.size);
                match (constraints.width, constraints.height) {
                    (Number::Undefined, Number::Undefined) => {}
                    (Number::Defined(width), Number::Undefined) => {
                        size.height = width * size.height / size.width;
                        size.width = width;
                    }
                    (Number::Undefined, Number::Defined(height)) => {
                        size.width = height * size.width / size.height;
                        size.height = height;
                    }
                    (Number::Defined(width), Number::Defined(height)) => {
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
            let taffy_node = taffy.new_leaf(taffy_style, measure).unwrap();
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

    pub fn update_window(&mut self, window: &Window) {
        let taffy = &mut self.taffy;
        let node = self.window_nodes.entry(window.id()).or_insert_with(|| {
            taffy
                .new_node(taffy::style::Style::default(), &Vec::new())
                .unwrap()
        });

        taffy
            .set_style(
                *node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(window.physical_width() as f32),
                        height: taffy::style::Dimension::Points(window.physical_height() as f32),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    pub fn set_window_children(
        &mut self,
        window_id: WindowId,
        children: impl Iterator<Item = Entity>,
    ) {
        let taffy_node = self.window_nodes.get(&window_id).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_taffy.get(&e).unwrap())
            .collect::<Vec<taffy::node::Node>>();
        self.taffy.set_children(*taffy_node, &child_nodes).unwrap();
    }

    pub fn compute_window_layouts(&mut self) {
        for window_node in self.window_nodes.values() {
            self.taffy
                .compute_layout(*window_node, taffy::geometry::Size::undefined())
                .unwrap();
        }
    }

    /// Removes each entity from the internal map and then removes their associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(node) = self.entity_to_taffy.remove(&entity) {
                self.taffy.remove(node);
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
    TaffyError(taffy::Error),
}

#[allow(clippy::too_many_arguments)]
pub fn flex_node_system(
    windows: Res<Windows>,
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut flex_surface: ResMut<FlexSurface>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    node_query: Query<(Entity, &Style, Option<&CalculatedSize>), (With<Node>, Changed<Style>)>,
    full_node_query: Query<(Entity, &Style, Option<&CalculatedSize>), With<Node>>,
    changed_size_query: Query<
        (Entity, &Style, &CalculatedSize),
        (With<Node>, Changed<CalculatedSize>),
    >,
    children_query: Query<(Entity, &Children), (With<Node>, Changed<Children>)>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
    removed_nodes: RemovedComponents<Node>,
) {
    // update window root nodes
    for window in windows.iter() {
        flex_surface.update_window(window);
    }

    // assume one window for time being...
    let logical_to_physical_factor = windows.scale_factor(WindowId::primary());
    let scale_factor = logical_to_physical_factor * ui_scale.scale;

    if scale_factor_events.iter().next_back().is_some() || ui_scale.is_changed() {
        update_changed(&mut *flex_surface, scale_factor, full_node_query);
    } else {
        update_changed(&mut *flex_surface, scale_factor, node_query);
    }

    fn update_changed<F: WorldQuery>(
        flex_surface: &mut FlexSurface,
        scaling_factor: f64,
        query: Query<(Entity, &Style, Option<&CalculatedSize>), F>,
    ) {
        // update changed nodes
        for (entity, style, calculated_size) in &query {
            // TODO: remove node from old hierarchy if its root has changed
            if let Some(calculated_size) = calculated_size {
                flex_surface.upsert_leaf(entity, style, *calculated_size, scaling_factor);
            } else {
                flex_surface.upsert_node(entity, style, scaling_factor);
            }
        }
    }

    for (entity, style, calculated_size) in &changed_size_query {
        flex_surface.upsert_leaf(entity, style, *calculated_size, scale_factor);
    }

    // clean up removed nodes
    flex_surface.remove_entities(&removed_nodes);

    // update window children (for now assuming all Nodes live in the primary window)
    if let Some(primary_window) = windows.get_primary() {
        flex_surface.set_window_children(primary_window.id(), root_node_query.iter());
    }

    // update children
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
        if node.size != new_size {
            node.size = new_size;
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
