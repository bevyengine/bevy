mod convert;

use crate::{prelude::UiRootCamera, CalculatedSize, Node, Style, UiScale};
use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    query::{Changed, Or, With, Without, WorldQuery},
    system::{Query, Res, ResMut, Resource},
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::{UVec2, Vec2};
use bevy_render::camera::Camera;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{WindowId, WindowScaleFactorChanged, Windows};
use std::fmt;
use taffy::{number::Number, Taffy};

#[derive(Resource)]
pub struct FlexSurface {
    /// Maps UI entities to taffy nodes.
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    /// Maps UI root entities to taffy root nodes.
    ///
    /// The taffy root node is a special node that has only one child: the real root node
    /// present that is stored in `entity_to_taffy`. This means that two taffy nodes are
    /// maintained for each bevy_ui root node.
    root_nodes: HashMap<Entity, taffy::node::Node>,
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
            .finish()
    }
}

impl Default for FlexSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            root_nodes: Default::default(),
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

    pub fn update_root(&mut self, entity: &Entity, physical_size: &UVec2) {
        let taffy = &mut self.taffy;
        let node = self.root_nodes.entry(*entity).or_insert_with(|| {
            taffy
                .new_node(taffy::style::Style::default(), &Vec::new())
                .unwrap()
        });

        taffy
            .set_style(
                *node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(physical_size.x as f32),
                        height: taffy::style::Dimension::Points(physical_size.y as f32),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    pub fn update_root_children(&mut self, root_entities: impl Iterator<Item = Entity>) {
        for entity in root_entities {
            let root_node = self.root_nodes.get(&entity).unwrap();
            let root_entity_node = self.entity_to_taffy.get(&entity).unwrap();
            self.taffy
                .set_children(*root_node, &[*root_entity_node])
                .unwrap();
        }
    }

    pub fn compute_root_layouts(&mut self) {
        for root_node in self.root_nodes.values() {
            self.taffy
                .compute_layout(*root_node, taffy::geometry::Size::undefined())
                .unwrap();
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
    camera_query: Query<&Camera>,
    root_node_query: Query<(Entity, Option<&UiRootCamera>), (With<Node>, Without<Parent>)>,
    changed_node_query: Query<
        (Entity, &Style, Option<&CalculatedSize>),
        (With<Node>, Or<(Changed<Style>, Changed<CalculatedSize>)>),
    >,
    full_node_query: Query<(Entity, &Style, Option<&CalculatedSize>), With<Node>>,
    full_children_query: Query<&Children, With<Node>>,
    changed_children_query: Query<(Entity, &Children), (With<Node>, Changed<Children>)>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform)>,
) {
    let flex_roots = get_flex_roots(&windows, &ui_scale, &camera_query, &root_node_query);

    // update root nodes
    for flex_root in &flex_roots {
        let physical_size = flex_root.physical_rect.1 - flex_root.physical_rect.0;
        flex_surface.update_root(&flex_root.entity, &physical_size);
    }

    // TODO: check individual roots for changed scaling factor instead of updating all of them.
    let scale_factor_changed =
        scale_factor_events.iter().next_back().is_some() || ui_scale.is_changed();

    if scale_factor_changed {
        // update every single node because scaling factor changed.
        for flex_root in &flex_roots {
            update_changed_nodes_recursively(
                &mut flex_surface,
                flex_root.scaling_factor,
                &full_node_query,
                &full_children_query,
                &flex_root.entity,
            );
        }
    } else {
        // update only the nodes that were changed.
        for flex_root in &flex_roots {
            update_changed_nodes_recursively(
                &mut flex_surface,
                flex_root.scaling_factor,
                &changed_node_query,
                &full_children_query,
                &flex_root.entity,
            );
        }
    }

    // TODO: handle removed nodes

    // update root children (for now assuming all Nodes live in the primary window)
    flex_surface.update_root_children(root_node_query.iter().map(|(entity, _)| entity));

    // update children
    for (entity, children) in &changed_children_query {
        flex_surface.update_children(entity, children);
    }

    // compute layouts
    flex_surface.compute_root_layouts();

    for flex_root in &flex_roots {
        update_node_transforms_recursively(
            &flex_surface,
            &mut node_transform_query,
            &full_children_query,
            flex_root.scaling_factor,
            &flex_root.physical_rect.0,
            None,
            &flex_root.entity,
        );
    }
}

/// Recursively checks UI nodes for changes and updates them with their new values.
fn update_changed_nodes_recursively<F: WorldQuery>(
    flex_surface: &mut FlexSurface,
    scaling_factor: f64,
    node_query: &Query<(Entity, &Style, Option<&CalculatedSize>), F>,
    children_query: &Query<&Children, With<Node>>,
    entity: &Entity,
) {
    // update this entity if it's part of the node_query
    if let Ok((_, style, calculated_size)) = node_query.get(*entity) {
        update_changed_node(flex_surface, scaling_factor, entity, style, calculated_size);
    }

    // call this function recursively for all of this node's children
    if let Ok(children) = children_query.get(*entity) {
        for child in children {
            update_changed_nodes_recursively(
                flex_surface,
                scaling_factor,
                node_query,
                children_query,
                child,
            );
        }
    }
}

/// Updates the UI node in the flex surface with its new values.
fn update_changed_node(
    flex_surface: &mut FlexSurface,
    scaling_factor: f64,
    entity: &Entity,
    style: &Style,
    calculated_size: Option<&CalculatedSize>,
) {
    if let Some(calculated_size) = calculated_size {
        flex_surface.upsert_leaf(*entity, style, *calculated_size, scaling_factor);
    } else {
        flex_surface.upsert_node(*entity, style, scaling_factor);
    }
}

/// Recursively updates the transform of all UI nodes.
fn update_node_transforms_recursively(
    flex_surface: &FlexSurface,
    node_transform_query: &mut Query<(Entity, &mut Node, &mut Transform)>,
    children_query: &Query<&Children, With<Node>>,
    scaling_factor: f64,
    position_offset: &UVec2,
    parent: Option<&Entity>,
    entity: &Entity,
) {
    update_node_transform(
        flex_surface,
        node_transform_query,
        scaling_factor,
        position_offset,
        parent,
        entity,
    );

    // call this function recursively for all of this node's children
    if let Ok(children) = children_query.get(*entity) {
        for child in children {
            update_node_transforms_recursively(
                flex_surface,
                node_transform_query,
                children_query,
                scaling_factor,
                position_offset,
                Some(entity),
                child,
            );
        }
    }
}

/// Recalculates the node transform and update it if it changed.
fn update_node_transform(
    flex_surface: &FlexSurface,
    node_transform_query: &mut Query<(Entity, &mut Node, &mut Transform)>,
    scaling_factor: f64,
    position_offset: &UVec2,
    parent: Option<&Entity>,
    entity: &Entity,
) {
    let physical_to_logical_factor = 1. / scaling_factor;
    let to_logical = |v| (physical_to_logical_factor * v as f64) as f32;

    // PERF: try doing this incrementally
    if let Ok((entity, mut node, mut transform)) = node_transform_query.get_mut(*entity) {
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
        new_position.x =
            to_logical(layout.location.x + position_offset.x as f32 + layout.size.width / 2.0);
        new_position.y =
            to_logical(layout.location.y + position_offset.y as f32 + layout.size.height / 2.0);
        if let Some(parent) = parent {
            if let Ok(parent_layout) = flex_surface.get_layout(*parent) {
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

struct FlexRoot {
    entity: Entity,
    scaling_factor: f64,
    physical_rect: (UVec2, UVec2),
}

/// Returns the list of roots for all ui trees with their physical rect and scaling factor.
///
/// When [`UiRootCamera`] component exists for a given root node, information for rendering
/// on the full primary window will be returned instead.
fn get_flex_roots(
    windows: &Windows,
    ui_scale: &UiScale,
    camera_query: &Query<&Camera>,
    root_node_query: &Query<(Entity, Option<&UiRootCamera>), (With<Node>, Without<Parent>)>,
) -> Vec<FlexRoot> {
    root_node_query
        .iter()
        .map(|(entity, ui_root)| {
            ui_root
                .and_then(|ui_root| camera_query.get(ui_root.0).ok())
                .map(|camera| FlexRoot {
                    entity,
                    scaling_factor: camera.target_scaling_factor().unwrap_or(1.0) * ui_scale.scale,
                    // TODO: make sure this won't explode and it makes sense in system ordering.
                    physical_rect: camera.physical_viewport_rect().unwrap_or((UVec2::default(), UVec2::default())),
                })
                .or_else(|| {
                    windows.get_primary().map(|window| FlexRoot {
                        entity,
                        scaling_factor: window.scale_factor() * ui_scale.scale,
                        physical_rect: (
                            UVec2::new(0, 0),
                            UVec2::new(window.physical_width(), window.physical_height()),
                        ),
                    })
                })
                .unwrap()
        })
        .collect::<Vec<_>>()
}
