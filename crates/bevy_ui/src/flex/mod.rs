mod convert;

use crate::{CalculatedSize, ControlNode, Node, Style};
use bevy_app::EventReader;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, FilterFetch, With, Without, WorldQuery},
    system::{Query, Res, ResMut},
};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_transform::prelude::{Children, Parent, Transform};
use bevy_utils::HashMap;
use bevy_window::{Window, WindowId, WindowScaleFactorChanged, Windows};
use std::fmt;
use stretch::{number::Number, Stretch};

pub struct FlexSurface {
    entity_to_stretch: HashMap<Entity, stretch::node::Node>,
    window_nodes: HashMap<WindowId, stretch::node::Node>,
    stretch: Stretch,
}

// SAFE: as long as MeasureFunc is Send + Sync. https://github.com/vislyhq/stretch/issues/69
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

fn _assert_send_sync_flex_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, stretch::node::Node>>();
    _assert_send_sync::<HashMap<WindowId, stretch::node::Node>>();
    // FIXME https://github.com/vislyhq/stretch/issues/69
    // _assert_send_sync::<Stretch>();
}

impl fmt::Debug for FlexSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FlexSurface")
            .field("entity_to_stretch", &self.entity_to_stretch)
            .field("window_nodes", &self.window_nodes)
            .finish()
    }
}

impl Default for FlexSurface {
    fn default() -> Self {
        Self {
            entity_to_stretch: Default::default(),
            window_nodes: Default::default(),
            stretch: Stretch::new(),
        }
    }
}

impl FlexSurface {
    pub fn upsert_node(&mut self, entity: Entity, style: &Style, scale_factor: f64) {
        let mut added = false;
        let stretch = &mut self.stretch;
        let stretch_style = convert::from_style(scale_factor, style);
        let stretch_node = self.entity_to_stretch.entry(entity).or_insert_with(|| {
            added = true;
            stretch.new_node(stretch_style, Vec::new()).unwrap()
        });

        if !added {
            self.stretch
                .set_style(*stretch_node, stretch_style)
                .unwrap();
        }
    }

    pub fn upsert_leaf(
        &mut self,
        entity: Entity,
        style: &Style,
        calculated_size: CalculatedSize,
        scale_factor: f64,
    ) {
        let stretch = &mut self.stretch;
        let stretch_style = convert::from_style(scale_factor, style);
        let measure = Box::new(move |constraints: stretch::geometry::Size<Number>| {
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
            Ok(size)
        });

        if let Some(stretch_node) = self.entity_to_stretch.get(&entity) {
            self.stretch
                .set_style(*stretch_node, stretch_style)
                .unwrap();
            self.stretch
                .set_measure(*stretch_node, Some(measure))
                .unwrap();
        } else {
            let stretch_node = stretch.new_leaf(stretch_style, measure).unwrap();
            self.entity_to_stretch.insert(entity, stretch_node);
        }
    }

    pub fn update_children(
        &mut self,
        entity: Entity,
        children: &Children,
        control_node_query: &mut Query<&mut ControlNode>,
        unfiltered_children_query: &Query<&Children>,
    ) {
        let mut stretch_children = Vec::with_capacity(children.len());
        fn inner(
            true_parent: Entity,
            child: Entity,
            control_node_query: &mut Query<&mut ControlNode>,
            unfiltered_children_query: &Query<&Children>,
            do_on_real: &mut impl FnMut(Entity),
        ) {
            if let Ok(mut control_node) = control_node_query.get_mut(child) {
                control_node.true_parent = Some(true_parent);
                for &child in unfiltered_children_query
                    .get(child)
                    .ok()
                    .into_iter()
                    .map(|c| &**c)
                    .flatten()
                {
                    inner(
                        true_parent,
                        child,
                        control_node_query,
                        unfiltered_children_query,
                        do_on_real,
                    );
                }
            } else {
                do_on_real(child);
            }
        }

        for &child in children.iter() {
            inner(
                entity,
                child,
                control_node_query,
                unfiltered_children_query,
                &mut |e| {
                    if let Some(stretch_node) = self.entity_to_stretch.get(&e) {
                        stretch_children.push(*stretch_node);
                    } else {
                        warn!(
                            "Unstyled child in a UI entity hierarchy. You are using an entity \
    without UI components as a child of an entity with UI components, results may be unexpected."
                        );
                    }
                },
            );
        }

        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch
            .set_children(*stretch_node, stretch_children)
            .unwrap();
    }

    pub fn update_window(&mut self, window: &Window) {
        let stretch = &mut self.stretch;
        let node = self.window_nodes.entry(window.id()).or_insert_with(|| {
            stretch
                .new_node(stretch::style::Style::default(), Vec::new())
                .unwrap()
        });

        stretch
            .set_style(
                *node,
                stretch::style::Style {
                    size: stretch::geometry::Size {
                        width: stretch::style::Dimension::Points(window.physical_width() as f32),
                        height: stretch::style::Dimension::Points(window.physical_height() as f32),
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
        let stretch_node = self.window_nodes.get(&window_id).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_stretch.get(&e).unwrap())
            .collect::<Vec<stretch::node::Node>>();
        self.stretch
            .set_children(*stretch_node, child_nodes)
            .unwrap();
    }

    pub fn compute_window_layouts(&mut self) {
        for window_node in self.window_nodes.values() {
            self.stretch
                .compute_layout(*window_node, stretch::geometry::Size::undefined())
                .unwrap();
        }
    }

    pub fn get_layout(&self, entity: Entity) -> Result<&stretch::result::Layout, FlexError> {
        if let Some(stretch_node) = self.entity_to_stretch.get(&entity) {
            self.stretch
                .layout(*stretch_node)
                .map_err(FlexError::StretchError)
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
    StretchError(stretch::Error),
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn flex_node_system(
    windows: Res<Windows>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut flex_surface: ResMut<FlexSurface>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    node_query: Query<(Entity, &Style, Option<&CalculatedSize>), (With<Node>, Changed<Style>)>,
    full_node_query: Query<(Entity, &Style, Option<&CalculatedSize>), With<Node>>,
    changed_size_query: Query<
        (Entity, &Style, &CalculatedSize),
        (With<Node>, Changed<CalculatedSize>),
    >,
    changed_children_query: Query<(Entity, &Children), (With<Node>, Changed<Children>)>,
    unfiltered_children_query: Query<&Children>,
    mut control_node_query: Query<&mut ControlNode>,
    changed_cnc_query: Query<Entity, (Changed<Children>, With<ControlNode>)>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
) {
    // update window root nodes
    for window in windows.iter() {
        flex_surface.update_window(window);
    }

    // assume one window for time being...
    let logical_to_physical_factor = if let Some(primary_window) = windows.get_primary() {
        primary_window.scale_factor()
    } else {
        1.
    };

    if scale_factor_events.iter().next_back().is_some() {
        update_changed(
            &mut *flex_surface,
            logical_to_physical_factor,
            full_node_query,
        );
    } else {
        update_changed(&mut *flex_surface, logical_to_physical_factor, node_query);
    }

    fn update_changed<F: WorldQuery>(
        flex_surface: &mut FlexSurface,
        scaling_factor: f64,
        query: Query<(Entity, &Style, Option<&CalculatedSize>), F>,
    ) where
        F::Fetch: FilterFetch,
    {
        // update changed nodes
        for (entity, style, calculated_size) in query.iter() {
            // TODO: remove node from old hierarchy if its root has changed
            if let Some(calculated_size) = calculated_size {
                flex_surface.upsert_leaf(entity, style, *calculated_size, scaling_factor);
            } else {
                flex_surface.upsert_node(entity, style, scaling_factor);
            }
        }
    }

    for (entity, style, calculated_size) in changed_size_query.iter() {
        flex_surface.upsert_leaf(entity, style, *calculated_size, logical_to_physical_factor);
    }

    // TODO: handle removed nodes

    // update window children (for now assuming all Nodes live in the primary window)
    if let Some(primary_window) = windows.get_primary() {
        flex_surface.set_window_children(primary_window.id(), root_node_query.iter());
    }

    // update children
    for (entity, children) in changed_children_query.iter() {
        flex_surface.update_children(
            entity,
            children,
            &mut control_node_query,
            &unfiltered_children_query,
        );
    }

    for entity in changed_cnc_query.iter() {
        let true_parent = if let Some(e) = control_node_query.get_mut(entity).unwrap().true_parent {
            e
        } else {
            continue;
        };
        let children = unfiltered_children_query.get(true_parent).unwrap();
        flex_surface.update_children(
            true_parent,
            children,
            &mut control_node_query,
            &unfiltered_children_query,
        );
    }

    // compute layouts
    flex_surface.compute_window_layouts();

    let physical_to_logical_factor = 1. / logical_to_physical_factor;

    let to_logical = |v| (physical_to_logical_factor * v as f64) as f32;

    // PERF: try doing this incrementally
    for (entity, mut node, mut transform, parent) in node_transform_query.iter_mut() {
        let layout = flex_surface.get_layout(entity).unwrap();
        node.size = Vec2::new(
            to_logical(layout.size.width),
            to_logical(layout.size.height),
        );
        let position = &mut transform.translation;
        position.x = to_logical(layout.location.x + layout.size.width / 2.0);
        position.y = to_logical(layout.location.y + layout.size.height / 2.0);
        if let Some(parent) = parent {
            let parent = control_node_query
                .get_mut(parent.0)
                .map(|cn| cn.true_parent.unwrap())
                .unwrap_or(parent.0);
            if let Ok(parent_layout) = flex_surface.get_layout(parent) {
                position.x -= to_logical(parent_layout.size.width / 2.0);
                position.y -= to_logical(parent_layout.size.height / 2.0);
            }
        }
    }
}
