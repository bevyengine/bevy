mod convert;

use crate::{Node, Style, ZIndex};

use bevy_app::EventReader;
use bevy_ecs::{Added, Changed, Entity, Mutated, Or, Query, Res, ResMut, With};
use bevy_log::{trace, warn};

use bevy_math::Vec2;

use bevy_text::CalculatedSize;
use bevy_transform::prelude::{Children, Parent, Transform};
use bevy_utils::HashMap;
use bevy_window::{Window, WindowId, WindowResized, Windows};
use std::{collections::hash_map::Entry, fmt};
use stretch::{number::Number, Stretch};

pub struct FlexSurface {
    entity_to_stretch: HashMap<Entity, stretch::node::Node>,
    window_nodes: HashMap<Entity, stretch::node::Node>,
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

#[derive(Default, Debug)]
pub struct NodeWindowId(WindowId);

// SAFE: as long as MeasureFunc is Send + Sync. https://github.com/vislyhq/stretch/issues/69
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

pub fn layout_system(
    mut flex_surface: ResMut<FlexSurface>,
    changed_style_query: Query<(Entity, &Style, Option<&CalculatedSize>), Changed<Style>>,
    mutated_size_query: Query<(Entity, &CalculatedSize), (With<Style>, Mutated<CalculatedSize>)>,
    mutated_children_query: Query<
        (Entity, &Children),
        (With<Style>, Or<(Added<Style>, Mutated<Children>)>),
    >,
    windows: Res<Windows>,
    new_window_nodes_query: Query<(Entity, &Style), (Or<(Added<Style>, Changed<Parent>)>,)>,
    is_root_query: Query<Option<&Parent>, With<Style>>,
    mut window_resized_events: EventReader<WindowResized>,
    window_id_query: Query<&NodeWindowId, With<Style>>,
    mut transform_xy_query: Query<
        (Entity, &mut Node, &mut Transform, Option<&Parent>),
        With<Style>,
    >,
) {
    let flex_surface = &mut *flex_surface;

    upsert_node_styles(flex_surface, &changed_style_query);

    update_node_sizes(flex_surface, &mutated_size_query);

    update_node_children(flex_surface, &mutated_children_query);

    upsert_window_nodes(
        flex_surface,
        &windows,
        &new_window_nodes_query,
        &is_root_query,
    );

    update_window_sizes(
        flex_surface,
        &*windows,
        &mut window_resized_events,
        &window_id_query,
    );

    compute_window_node_layouts(flex_surface);

    update_transforms_xy(flex_surface, &mut transform_xy_query);
}

fn upsert_node_styles(
    flex_surface: &mut FlexSurface,
    changed_style_query: &Query<(Entity, &Style, Option<&CalculatedSize>), Changed<Style>>,
) {
    for (entity, style, size) in changed_style_query.iter() {
        match flex_surface.entity_to_stretch.entry(entity) {
            Entry::Occupied(entry) => {
                trace!("Updating style for {:?}", entity);
                flex_surface
                    .stretch
                    .set_style(*entry.get(), convert::from_style(style))
                    .unwrap();
            }
            Entry::Vacant(entry) => {
                trace!("Inserting stretch node for: {:?}", entity);
                let stretch_style = convert::from_style(&style);
                let node = if let Some(&size) = size {
                    let measure = Box::new(move |constraints| leaf_measure(constraints, size));
                    flex_surface
                        .stretch
                        .new_leaf(stretch_style, measure)
                        .unwrap()
                } else {
                    flex_surface
                        .stretch
                        .new_node(stretch_style, Vec::new())
                        .unwrap()
                };
                entry.insert(node);
            }
        }
    }
}

fn leaf_measure(
    constraints: stretch::geometry::Size<Number>,
    size: CalculatedSize,
) -> Result<stretch::geometry::Size<f32>, Box<dyn std::any::Any>> {
    let mut size = convert::from_f32_size(size.size);
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
}

fn update_node_sizes(
    flex_surface: &mut FlexSurface,
    mutated_size_query: &Query<(Entity, &CalculatedSize), (With<Style>, Mutated<CalculatedSize>)>, // TODO: Not<Changed<Style>>
) {
    for (entity, &size) in mutated_size_query.iter() {
        trace!("Updating size for {:?}", entity);
        let node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let measure = Box::new(move |constraints| leaf_measure(constraints, size));
        flex_surface
            .stretch
            .set_measure(*node, Some(measure))
            .unwrap();
    }
    for entity in mutated_size_query.removed::<CalculatedSize>().iter() {
        trace!("Removing size for {:?}", entity);
        let node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        flex_surface.stretch.set_measure(*node, None).unwrap();
    }
}

fn update_node_children(
    flex_surface: &mut FlexSurface,
    mutated_children_query: &Query<
        (Entity, &Children),
        (With<Style>, Or<(Added<Style>, Mutated<Children>)>),
    >,
) -> () {
    for (entity, children) in mutated_children_query.iter() {
        trace!("Updating children for: {:?}", entity);
        let parent_node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let children_nodes = children
            .iter()
            .filter_map(|child| flex_surface.entity_to_stretch.get(&child).copied())
            .collect();
        flex_surface
            .stretch
            .set_children(*parent_node, children_nodes)
            .unwrap();
    }
}

fn upsert_window_nodes(
    flex_surface: &mut FlexSurface,
    windows: &Windows,
    new_window_nodes_query: &Query<(Entity, &Style), (Or<(Added<Style>, Changed<Parent>)>,)>,
    is_root_query: &Query<Option<&Parent>, With<Style>>,
) {
    let stretch = &mut flex_surface.stretch;
    flex_surface.window_nodes.retain(|entity, window_node| {
        if is_node_root(*entity, is_root_query) {
            true
        } else {
            trace!("Removing window node for: {:?}", entity);
            stretch.remove(*window_node);
            false
        }
    });

    for (entity, _style) in new_window_nodes_query.iter() {
        if is_node_root(entity, is_root_query) {
            trace!("Adding window node for: {:?}", entity);
            let window = windows.get_primary().unwrap();
            let entity_node = flex_surface.entity_to_stretch.get(&entity).unwrap();

            let window_node = stretch
                .new_node(Default::default(), vec![*entity_node])
                .unwrap();
            set_window_node_style(stretch, window_node, window);
            if let Some(previous) = flex_surface.window_nodes.insert(entity, window_node) {
                warn!("Repacing UI window node for: {:?}", entity);
                stretch.remove(previous);
            }
        }
    }
}

fn is_node_root(entity: Entity, query: &Query<Option<&Parent>, With<Style>>) -> bool {
    if let Some(parent) = query.get(entity).unwrap() {
        if query.get(**parent).is_err() {
            // Parent has no Style
            true
        } else {
            false
        }
    } else {
        // There is no parent
        true
    }
}

fn set_window_node_style(stretch: &mut Stretch, window_node: stretch::node::Node, window: &Window) {
    stretch
        .set_style(
            window_node,
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

fn update_window_sizes(
    flex_surface: &mut FlexSurface,
    windows: &Windows,
    window_resized_events: &mut EventReader<WindowResized>,
    window_id_query: &Query<&NodeWindowId, With<Style>>,
) {
    for event in window_resized_events.iter() {
        let resized_window_id = event.id;
        for (entity, window_node) in flex_surface.window_nodes.iter() {
            let entity_window_id = window_id_query
                .get(*entity)
                .map(|node_window_id| node_window_id.0)
                .unwrap_or_else(|_| (windows.get_primary().unwrap().id()));
            if resized_window_id == entity_window_id {
                trace!(
                    "Rescaling window node for {:?} ({:?})",
                    entity,
                    resized_window_id
                );
                set_window_node_style(
                    &mut flex_surface.stretch,
                    *window_node,
                    windows.get(resized_window_id).unwrap(),
                );
            }
        }
    }
}

fn compute_window_node_layouts(flex_surface: &mut FlexSurface) {
    for window_node in flex_surface.window_nodes.values() {
        flex_surface
            .stretch
            .compute_layout(*window_node, stretch::geometry::Size::undefined())
            .unwrap();
    }
}

fn update_transforms_xy(
    flex_surface: &mut FlexSurface,
    transform_xy_query: &mut Query<
        (Entity, &mut Node, &mut Transform, Option<&Parent>),
        With<Style>,
    >,
) {
    for (entity, mut node, mut transform, parent) in transform_xy_query.iter_mut() {
        let stretch_node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let layout = flex_surface.stretch.layout(*stretch_node).unwrap();
        node.size = Vec2::new(layout.size.width, layout.size.height);

        let position = &mut transform.translation;
        position.x = layout.location.x + layout.size.width / 2.0;
        position.y = layout.location.y + layout.size.height / 2.0;
        if let Some(parent) = parent {
            if let Some(stretch_node) = flex_surface.entity_to_stretch.get(&**parent) {
                let parent_layout = flex_surface.stretch.layout(*stretch_node).unwrap();

                position.x -= parent_layout.size.width / 2.0;
                position.y -= parent_layout.size.height / 2.0;
            }
        }
    }
}

pub fn z_index_system(
    changed_zindex_query: Query<
        Entity,
        (
            With<Style>,
            With<Transform>,
            Or<(Changed<Style>, Changed<Parent>, Changed<Children>)>,
        ),
    >,
    mut transform_z_query: Query<(&Style, Option<&Children>, Option<&Parent>, &mut Transform)>,
) {
    for entity in changed_zindex_query.iter() {
        update_one_transform_z(entity, &mut transform_z_query);
    }
}

fn update_one_transform_z(
    entity: Entity,
    transform_z_query: &mut Query<(&Style, Option<&Children>, Option<&Parent>, &mut Transform)>,
) {
    trace!("Calculating z transforms for entity {:?}", entity);
    // Find the origin of this stacking context
    let z_index = transform_z_query
        .get_component::<Style>(entity)
        .expect("Root UI entity cannot have z_index = ZIndex::Auto")
        .z_index;
    if z_index == ZIndex::Auto {
        if let Ok(parent) = transform_z_query.get_component::<Parent>(entity) {
            return update_one_transform_z(**parent, transform_z_query);
        }
    }

    // Find all entities that are in this stacking context
    let mut stacking_context = Vec::new();
    fill_stacking_context(entity, &mut stacking_context, transform_z_query);

    stacking_context.sort_by_key(|(_entity, z_index)| *z_index);

    // Set their z transform evenly spaced in 0..N
    for (i, (entity, _z_index)) in stacking_context.iter().enumerate() {
        let mut transform = transform_z_query
            .get_component_mut::<Transform>(*entity)
            .unwrap();
        transform.translation.z = (i + 1) as f32;
    }
    // Set the scale of the origin so that other entities are effectively spaced in 0..1 in the origin's local transform
    let mut transform = transform_z_query
        .get_component_mut::<Transform>(entity)
        .unwrap();
    transform.scale.z = 1.0 / (stacking_context.len() + 1) as f32;
}

fn fill_stacking_context(
    entity: Entity,
    stacking_context: &mut Vec<(Entity, i16)>,
    transform_z_query: &Query<(&Style, Option<&Children>, Option<&Parent>, &mut Transform)>,
) {
    if let Ok(children) = transform_z_query.get_component::<Children>(entity) {
        for &child in children.iter() {
            let z_index = &transform_z_query
                .get_component::<Style>(child)
                .unwrap()
                .z_index;
            match z_index {
                ZIndex::None => stacking_context.push((child, 0)),
                ZIndex::Auto => {
                    stacking_context.push((child, 0));
                    fill_stacking_context(child, stacking_context, transform_z_query);
                }
                ZIndex::Some(i) => stacking_context.push((child, *i)),
            };
        }
    }
}

pub fn garbage_collection_system(
    mut flex_surface: ResMut<FlexSurface>,
    style_query: Query<&Style>,
) {
    let flex_surface = &mut *flex_surface;
    let stretch = &mut flex_surface.stretch;
    let entity_to_stretch = &mut flex_surface.entity_to_stretch;
    entity_to_stretch.retain(|entity, node| {
        if style_query.get(*entity).is_ok() {
            true
        } else {
            trace!("Removing stretch node for: {:?}", entity);
            stretch.remove(*node);
            false
        }
    });
}
