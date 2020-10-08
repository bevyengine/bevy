mod convert;

use crate::{CalculatedSize, Node, Style};
use bevy_ecs::{Changed, Entity, Query, Res, ResMut, With, Without};
use bevy_math::Vec2;
use bevy_transform::prelude::{Children, Parent, Transform};
use bevy_utils::HashMap;
use bevy_window::{Window, WindowId, Windows};
use std::fmt;
use stretch::{number::Number, Stretch};

pub struct FlexSurface {
    entity_to_stretch: HashMap<Entity, stretch::node::Node>,
    window_nodes: HashMap<WindowId, stretch::node::Node>,
    stretch: Stretch,
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
    pub fn upsert_node(&mut self, entity: Entity, style: &Style) {
        let mut added = false;
        let stretch = &mut self.stretch;
        let stretch_style = style.into();
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

    pub fn upsert_leaf(&mut self, entity: Entity, style: &Style, calculated_size: CalculatedSize) {
        let stretch = &mut self.stretch;
        let stretch_style = style.into();
        let measure = Box::new(move |constraints: stretch::geometry::Size<Number>| {
            let mut size = stretch::geometry::Size {
                width: calculated_size.size.width,
                height: calculated_size.size.height,
            };
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

    pub fn update_children(&mut self, entity: Entity, children: &Children) {
        let mut stretch_children = Vec::with_capacity(children.len());
        for child in children.iter() {
            let stretch_node = self.entity_to_stretch.get(child).unwrap();
            stretch_children.push(*stretch_node);
        }

        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch
            .set_children(*stretch_node, stretch_children)
            .unwrap();
    }

    pub fn update_window(&mut self, window: &Window) {
        let stretch = &mut self.stretch;
        let node = self.window_nodes.entry(window.id).or_insert_with(|| {
            stretch
                .new_node(stretch::style::Style::default(), Vec::new())
                .unwrap()
        });

        stretch
            .set_style(
                *node,
                stretch::style::Style {
                    size: stretch::geometry::Size {
                        width: stretch::style::Dimension::Points(window.width as f32),
                        height: stretch::style::Dimension::Points(window.height as f32),
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

    pub fn get_layout(&self, entity: Entity) -> Result<&stretch::result::Layout, stretch::Error> {
        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch.layout(*stretch_node)
    }
}

// SAFE: as long as MeasureFunc is Send + Sync. https://github.com/vislyhq/stretch/issues/69
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

pub fn flex_node_system(
    windows: Res<Windows>,
    mut flex_surface: ResMut<FlexSurface>,
    mut root_node_query: Query<With<Node, Without<Parent, Entity>>>,
    mut node_query: Query<With<Node, (Entity, Changed<Style>, Option<&CalculatedSize>)>>,
    mut changed_size_query: Query<With<Node, (Entity, &Style, Changed<CalculatedSize>)>>,
    mut children_query: Query<With<Node, (Entity, Changed<Children>)>>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
) {
    // update window root nodes
    for window in windows.iter() {
        flex_surface.update_window(window);
    }

    // update changed nodes
    for (entity, style, calculated_size) in &mut node_query.iter() {
        // TODO: remove node from old hierarchy if its root has changed
        if let Some(calculated_size) = calculated_size {
            flex_surface.upsert_leaf(entity, &style, *calculated_size);
        } else {
            flex_surface.upsert_node(entity, &style);
        }
    }

    for (entity, style, calculated_size) in &mut changed_size_query.iter() {
        flex_surface.upsert_leaf(entity, &style, *calculated_size);
    }

    // TODO: handle removed nodes

    // update window children (for now assuming all Nodes live in the primary window)
    if let Some(primary_window) = windows.get_primary() {
        flex_surface.set_window_children(primary_window.id, root_node_query.iter().iter());
    }

    // update children
    for (entity, children) in &mut children_query.iter() {
        flex_surface.update_children(entity, &children);
    }

    // compute layouts
    flex_surface.compute_window_layouts();

    for (entity, mut node, mut transform, parent) in &mut node_transform_query.iter() {
        let layout = flex_surface.get_layout(entity).unwrap();
        node.size = Vec2::new(layout.size.width, layout.size.height);
        let position = transform.translation_mut();
        position.set_x(layout.location.x + layout.size.width / 2.0);
        position.set_y(layout.location.y + layout.size.height / 2.0);
        if let Some(parent) = parent {
            if let Ok(parent_layout) = flex_surface.get_layout(parent.0) {
                *position.x_mut() -= parent_layout.size.width / 2.0;
                *position.y_mut() -= parent_layout.size.height / 2.0;
            }
        }
    }
}
