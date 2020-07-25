use crate::Node;
use bevy_ecs::{Changed, Entity, Query, Res, ResMut, With, Without};
use bevy_math::Vec2;
use bevy_transform::prelude::{Children, LocalTransform, Parent};
use bevy_window::Windows;
use std::collections::{HashMap, HashSet};
use stretch::{
    geometry::Size,
    number::Number,
    result::Layout,
    style::{Dimension, PositionType, Style},
    Stretch,
};

#[derive(Default, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct FlexSurfaceId(usize);

#[derive(Default)]
pub struct FlexSurfaces {
    surfaces: HashMap<FlexSurfaceId, FlexSurface>,
}

pub struct FlexSurface {
    entity_to_stretch: HashMap<Entity, stretch::node::Node>,
    stretch_to_entity: HashMap<stretch::node::Node, Entity>,
    surface_root_node: stretch::node::Node,
    size: Vec2,
    stretch: Stretch,
    orphans: HashSet<Entity>,
}

impl FlexSurface {
    fn new() -> Self {
        let mut stretch = Stretch::new();
        let surface_root_node = stretch
            .new_node(
                Style {
                    size: Size {
                        width: Dimension::Percent(1.0),
                        height: Dimension::Percent(1.0),
                    },
                    ..Default::default()
                },
                Vec::new(),
            )
            .unwrap();
        Self {
            entity_to_stretch: Default::default(),
            stretch_to_entity: Default::default(),
            orphans: Default::default(),
            size: Default::default(),
            stretch,
            surface_root_node,
        }
    }

    pub fn upsert_node(&mut self, entity: Entity, style: &Style, orphan: bool) {
        let mut added = false;
        let stretch = &mut self.stretch;
        let stretch_to_entity = &mut self.stretch_to_entity;
        let stretch_node = self.entity_to_stretch.entry(entity).or_insert_with(|| {
            added = true;
            let stretch_node = stretch.new_node(style.clone(), Vec::new()).unwrap();
            stretch_to_entity.insert(stretch_node, entity);
            stretch_node
        });

        if !added {
            self.stretch
                .set_style(*stretch_node, style.clone())
                .unwrap();
        }

        if orphan && !self.orphans.contains(&entity) {
            self.stretch
                .add_child(self.surface_root_node, *stretch_node)
                .unwrap();
            self.orphans.insert(entity);
        } else if !orphan && self.orphans.contains(&entity) {
            self.stretch
                .remove_child(self.surface_root_node, *stretch_node)
                .unwrap();
            self.orphans.remove(&entity);
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

    pub fn compute_layout(&mut self) {
        self.stretch
            .compute_layout(
                self.surface_root_node,
                stretch::geometry::Size {
                    width: Number::Defined(self.size.x()),
                    height: Number::Defined(self.size.y()),
                },
            )
            .unwrap();
    }

    pub fn get_layout(&self, entity: Entity) -> Result<&Layout, stretch::Error> {
        let stretch_node = self.entity_to_stretch.get(&entity).unwrap();
        self.stretch.layout(*stretch_node)
    }
}

// SAFE: as long as MeasureFunc is Send + Sync. https://github.com/vislyhq/stretch/issues/69
unsafe impl Send for FlexSurfaces {}
unsafe impl Sync for FlexSurfaces {}

pub fn primary_window_flex_surface_system(
    windows: Res<Windows>,
    mut flex_surfaces: ResMut<FlexSurfaces>,
) {
    if let Some(surface) = flex_surfaces.surfaces.get_mut(&FlexSurfaceId::default()) {
        if let Some(window) = windows.get_primary() {
            surface.size = Vec2::new(window.width as f32, window.height as f32);
        }
    }
}

pub fn flex_node_system(
    mut flex_surfaces: ResMut<FlexSurfaces>,
    mut root_node_query: Query<With<Node, Without<Parent, (&FlexSurfaceId, &mut Style)>>>,
    mut node_query: Query<With<Node, (Entity, &FlexSurfaceId, Changed<Style>, Option<&Parent>)>>,
    mut children_query: Query<With<Node, (Entity, &FlexSurfaceId, Changed<Children>)>>,
    mut node_transform_query: Query<(
        Entity,
        &mut Node,
        &FlexSurfaceId,
        &mut LocalTransform,
        Option<&Parent>,
    )>,
) {
    // initialize stretch hierarchies
    for (flex_surface_id, mut style) in &mut root_node_query.iter() {
        flex_surfaces
            .surfaces
            .entry(*flex_surface_id)
            .or_insert_with(|| FlexSurface::new());

        // root nodes should not be positioned relative to each other
        style.position_type = PositionType::Absolute;
    }

    // TODO: cleanup unused surfaces

    // update changed nodes
    for (entity, flex_surface_id, style, parent) in &mut node_query.iter() {
        // TODO: remove node from old hierarchy if its root has changed
        let surface = flex_surfaces.surfaces.get_mut(flex_surface_id).unwrap();
        surface.upsert_node(entity, &style, parent.is_none());
    }

    // TODO: handle removed nodes

    // update children
    for (entity, flex_surface_id, children) in &mut children_query.iter() {
        let surface = flex_surfaces.surfaces.get_mut(flex_surface_id).unwrap();
        surface.update_children(entity, &children);
    }

    // compute layouts
    for surface in flex_surfaces.surfaces.values_mut() {
        surface.compute_layout();
    }

    for (entity, mut node, flex_surface_id, mut local, parent) in &mut node_transform_query.iter() {
        let surface = flex_surfaces.surfaces.get_mut(flex_surface_id).unwrap();
        let layout = surface.get_layout(entity).unwrap();
        node.size = Vec2::new(layout.size.width, layout.size.height);
        let mut position = local.w_axis();
        position.set_x(layout.location.x + layout.size.width / 2.0);
        position.set_y(layout.location.y + layout.size.height / 2.0);
        if let Some(parent) = parent {
            if let Ok(parent_layout) = surface.get_layout(parent.0) {
                *position.x_mut() -= parent_layout.size.width / 2.0;
                *position.y_mut() -= parent_layout.size.height / 2.0;
            }
        }

        local.set_w_axis(position);
    }
}
