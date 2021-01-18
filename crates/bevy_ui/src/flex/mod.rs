mod convert;

use crate::{Node, Style, ZIndex};

use bevy_app::EventReader;
use bevy_ecs::{Added, Changed, Entity, Mutated, Not, Or, Query, QuerySet, Res, ResMut, With};
use bevy_log::{trace, warn};

use bevy_math::Vec2;

use bevy_text::CalculatedSize;
use bevy_transform::{
    components::GlobalTransform,
    prelude::{Children, Parent, Transform},
};
use bevy_utils::HashMap;
use bevy_window::{Window, WindowId, WindowResized, Windows};
use std::{collections::hash_map::Entry, fmt};
use stretch::{number::Number, Stretch};

pub struct FlexSurface {
    entity_to_stretch: HashMap<Entity, stretch::node::Node>,
    window_nodes: WindowNodes,
    stretch: Stretch,
    stacking_contexts: HashMap<Entity, StackingContext>,
    root_stacking_context: StackingContext,
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
            stacking_contexts: Default::default(),
            root_stacking_context: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
struct WindowNodes {
    map: HashMap<Entity, stretch::node::Node>,
    dirty: bool,
}

#[derive(Debug, Default)]
struct StackingContext {
    // The root stacking context does not have a root entity
    root_entity: Option<Entity>,
    context: Vec<(Entity, i16)>,
    updated: bool,
}

#[derive(Default, Debug)]
pub struct NodeWindowId(pub WindowId);

// SAFE: as long as MeasureFunc is Send + Sync. https://github.com/vislyhq/stretch/issues/69
unsafe impl Send for FlexSurface {}
unsafe impl Sync for FlexSurface {}

#[allow(clippy::too_many_arguments)]
pub fn layout_system(
    mut flex_surface: ResMut<FlexSurface>,
    changed_style_query: Query<(Entity, &Style, Option<&CalculatedSize>), Changed<Style>>,
    mutated_size_query: Query<
        (Entity, &CalculatedSize),
        (With<Style>, Mutated<CalculatedSize>, Not<Changed<Style>>),
    >,
    mutated_children_query: Query<
        (Entity, &Children),
        (With<Style>, Or<(Added<Style>, Changed<Children>)>),
    >,
    windows: Res<Windows>,
    new_window_nodes_query: Query<
        (Entity, &Style, Option<&NodeWindowId>),
        Or<(Added<Style>, Changed<Parent>, Changed<NodeWindowId>)>,
    >,
    mut window_resized_events: EventReader<WindowResized>,
    window_id_query: Query<&NodeWindowId, With<Style>>,
    style_query: Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
    mut transform_queries: QuerySet<(
        Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
        Query<(&mut Transform, &mut GlobalTransform, Option<&Parent>), With<Style>>,
    )>,
    changed_zindex_query: Query<
        Entity,
        (
            With<Style>,
            Or<(Changed<Style>, Changed<Parent>, Changed<Children>)>,
        ),
    >,
) {
    trace!("Start of layout_system");
    let mut dirty_layout = false;
    let flex_surface = &mut *flex_surface;

    upsert_node_styles(flex_surface, &mut dirty_layout, &changed_style_query);
    update_node_sizes(flex_surface, &mut dirty_layout, &mutated_size_query);
    update_node_children(
        flex_surface,
        &mut dirty_layout,
        &mutated_children_query,
        &style_query,
    );
    upsert_window_nodes(
        flex_surface,
        &mut dirty_layout,
        &windows,
        &new_window_nodes_query,
        &style_query,
    );
    update_window_sizes(
        flex_surface,
        &mut dirty_layout,
        &*windows,
        &mut window_resized_events,
        &window_id_query,
    );
    if dirty_layout {
        compute_window_node_layouts(flex_surface);
        update_transforms_xy(flex_surface, &style_query, &mut transform_queries.q0_mut());
    }
    update_transforms_z(
        flex_surface,
        &changed_zindex_query,
        &style_query,
        &mut transform_queries.q1_mut(),
    );
}

fn upsert_node_styles(
    flex_surface: &mut FlexSurface,
    dirty_layout: &mut bool,
    changed_style_query: &Query<(Entity, &Style, Option<&CalculatedSize>), Changed<Style>>,
) {
    for (entity, style, size) in changed_style_query.iter() {
        *dirty_layout = true;
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
                    let measure = Box::new(move |constraints| Ok(leaf_measure(constraints, size)));
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
) -> stretch::geometry::Size<f32> {
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
    size
}

fn update_node_sizes(
    flex_surface: &mut FlexSurface,
    dirty_layout: &mut bool,
    mutated_size_query: &Query<
        (Entity, &CalculatedSize),
        (With<Style>, Mutated<CalculatedSize>, Not<Changed<Style>>),
    >,
) {
    for (entity, &size) in mutated_size_query.iter() {
        trace!("Updating size for {:?}", entity);
        *dirty_layout = true;
        let node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let measure = Box::new(move |constraints| Ok(leaf_measure(constraints, size)));
        flex_surface
            .stretch
            .set_measure(*node, Some(measure))
            .unwrap();
    }
    for entity in mutated_size_query.removed::<CalculatedSize>().iter() {
        trace!("Removing size for {:?}", entity);
        *dirty_layout = true;
        if let Some(node) = flex_surface.entity_to_stretch.get(&entity) {
            flex_surface.stretch.set_measure(*node, None).unwrap();
        }
    }
}

fn update_node_children(
    flex_surface: &mut FlexSurface,
    dirty_layout: &mut bool,
    mutated_children_query: &Query<
        (Entity, &Children),
        (With<Style>, Or<(Added<Style>, Changed<Children>)>),
    >,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
) {
    for (entity, children) in mutated_children_query.iter() {
        trace!("Updating children for: {:?}", entity);
        *dirty_layout = true;
        let parent_node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let children_nodes = children
            .iter()
            .filter_map(|child| {
                if !is_root_node(*child, style_query) {
                    flex_surface.entity_to_stretch.get(&child).copied()
                } else {
                    None
                }
            })
            .collect();
        flex_surface
            .stretch
            .set_children(*parent_node, children_nodes)
            .unwrap();
    }
}

fn upsert_window_nodes(
    flex_surface: &mut FlexSurface,
    dirty_layout: &mut bool,
    windows: &Windows,
    new_window_nodes_query: &Query<
        (Entity, &Style, Option<&NodeWindowId>),
        Or<(Added<Style>, Changed<Parent>, Changed<NodeWindowId>)>,
    >,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
) {
    let stretch = &mut flex_surface.stretch;
    let window_nodes_map = &mut flex_surface.window_nodes.map;
    let window_nodes_dirty = &mut flex_surface.window_nodes.dirty;
    window_nodes_map.retain(|entity, window_node| {
        if is_root_node(*entity, style_query) {
            true
        } else {
            trace!("Removing window node for: {:?}", entity);
            stretch.remove(*window_node);
            *window_nodes_dirty = true;
            false
        }
    });

    for (entity, _style, node_window_id) in new_window_nodes_query.iter() {
        if is_root_node(entity, style_query) {
            *dirty_layout = true;
            let window = if let Some(node_window_id) = node_window_id {
                trace!(
                    "Adding window node for: {:?} on {:?}",
                    entity,
                    node_window_id.0
                );
                windows.get(node_window_id.0).unwrap()
            } else {
                trace!("Adding window node for: {:?} on primary window", entity);
                windows.get_primary().unwrap()
            };
            let entity_node = flex_surface.entity_to_stretch.get(&entity).unwrap();

            let window_node = stretch
                .new_node(Default::default(), vec![*entity_node])
                .unwrap();
            set_window_node_style(stretch, window_node, window);
            if let Some(previous) = flex_surface.window_nodes.map.insert(entity, window_node) {
                warn!("Repacing UI window node for: {:?}", entity);
                stretch.remove(previous);
            }
            flex_surface.window_nodes.dirty = true;
        }
    }
}

fn is_root_node(
    entity: Entity,
    query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
) -> bool {
    // Entities with NodeWindowId are explicit roots
    if query.get_component::<NodeWindowId>(entity).is_ok() {
        return true;
    }
    // Otherwise, a root has no parent or its parent has no Style
    query
        .get_component::<Parent>(entity)
        .map(|parent| query.get(**parent).is_err())
        .unwrap_or(true)
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
    dirty_layout: &mut bool,
    windows: &Windows,
    window_resized_events: &mut EventReader<WindowResized>,
    window_id_query: &Query<&NodeWindowId, With<Style>>,
) {
    for event in window_resized_events.iter() {
        let resized_window_id = event.id;
        for (entity, window_node) in flex_surface.window_nodes.map.iter() {
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
                *dirty_layout = true;
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
    for (entity, window_node) in flex_surface.window_nodes.map.iter() {
        trace!("Computing layout for window node {:?}", entity);
        flex_surface
            .stretch
            .compute_layout(*window_node, stretch::geometry::Size::undefined())
            .unwrap();
    }
}

fn update_transforms_xy(
    flex_surface: &mut FlexSurface,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
    transform_query: &mut Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
) {
    trace!("Updating xy transforms");
    for (entity, mut node, mut transform, parent) in transform_query.iter_mut() {
        let stretch_node = flex_surface.entity_to_stretch.get(&entity).unwrap();
        let layout = flex_surface.stretch.layout(*stretch_node).unwrap();
        node.size = Vec2::new(layout.size.width, layout.size.height);

        let position = &mut transform.translation;
        position.x = layout.location.x + layout.size.width / 2.0;
        position.y = layout.location.y + layout.size.height / 2.0;

        if let Some(parent) = parent {
            if !is_root_node(entity, style_query) {
                let stretch_node = flex_surface.entity_to_stretch.get(&**parent).unwrap();
                let parent_layout = flex_surface.stretch.layout(*stretch_node).unwrap();

                position.x -= parent_layout.size.width / 2.0;
                position.y -= parent_layout.size.height / 2.0;
            }
        }
    }
}

// This should be run after transform_propagate_system and before drawing to properly place embedded window nodes
pub fn window_nodes_transform_system(
    flex_surface: ResMut<FlexSurface>,
    mut transform_query: Query<(&mut Transform, &mut GlobalTransform, Option<&Parent>)>,
    changed_parent_query: Query<(), Changed<GlobalTransform>>,
) {
    for &entity in flex_surface.window_nodes.map.keys() {
        let (transform, mut global_transform, parent) = transform_query.get_mut(entity).unwrap();
        if let Some(parent) = parent {
            if changed_parent_query.get(**parent).is_ok() {
                trace!(
                    "Updating GlobalTransform for embedded window node {:?}",
                    entity
                );
                // A root window node's Transform is absolute
                *global_transform = GlobalTransform::from(*transform);
            }
        }
    }
}

pub fn update_transforms_z(
    flex_surface: &mut FlexSurface,
    changed_zindex_query: &Query<
        Entity,
        (
            With<Style>,
            Or<(Changed<Style>, Changed<Parent>, Changed<Children>)>,
        ),
    >,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
    transform_query: &mut Query<
        (&mut Transform, &mut GlobalTransform, Option<&Parent>),
        With<Style>,
    >,
) {
    // Update the root stacking context if window nodes were added/removed, or if any window node has changed
    let mut dirty = flex_surface.window_nodes.dirty;
    flex_surface.window_nodes.dirty = false;
    for &entity in flex_surface.window_nodes.map.keys() {
        if changed_zindex_query.get(entity).is_ok() {
            dirty = true;
        }
    }
    if dirty {
        trace!("Updating root stacking context");
        let stacking_context = &mut flex_surface.root_stacking_context;
        stacking_context.updated = true;
        stacking_context.context.clear();
        for (&entity, _node) in flex_surface.window_nodes.map.iter() {
            let z_index = style_query.get_component::<Style>(entity).unwrap().z_index;
            stacking_context.context.push((entity, z_index.get()));
            if z_index == ZIndex::Auto {
                fill_stacking_context(entity, stacking_context, z_index.get(), style_query);
            }
            stacking_context
                .context
                .sort_by_key(|(_entity, z_index)| *z_index);
        }
    }

    for entity in changed_zindex_query.iter() {
        trace!("Updating stacking context for entity {:?}", entity);
        update_stacking_context(&mut *flex_surface, entity, style_query);
        dirty = true;
    }

    // Update z transforms if any stacking context has been updated
    if dirty {
        trace!("Updating z transforms");
        let mut current_z = 0.;
        update_transforms_z_impl(
            flex_surface,
            &flex_surface.root_stacking_context,
            &mut current_z,
            transform_query,
        );
        // Reset updated flag
        flex_surface.root_stacking_context.updated = false;
        for stacking_context in flex_surface.stacking_contexts.values_mut() {
            stacking_context.updated = false;
        }
    }
}

fn update_stacking_context(
    flex_surface: &mut FlexSurface,
    entity: Entity,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
) {
    let stacking_context_entry = flex_surface.stacking_contexts.entry(entity);

    // Entites with ZIndex::Auto and their children are included in the parent's context
    if !is_root_node(entity, style_query) {
        let z_index = style_query.get_component::<Style>(entity).unwrap().z_index;
        if z_index == ZIndex::Auto {
            let parent = style_query.get_component::<Parent>(entity).unwrap();
            if let Entry::Occupied(entry) = stacking_context_entry {
                entry.remove();
            }
            trace!(
                "{:?} has ZIndex::Auto, including it into its parent {:?}",
                entity,
                **parent
            );
            return update_stacking_context(flex_surface, **parent, style_query);
        }
    }

    let children = style_query.get_component::<Children>(entity);
    if children.is_err() || children.unwrap().is_empty() {
        trace!("Found leaf: {:?}", entity);
        if let Entry::Occupied(entry) = stacking_context_entry {
            entry.remove();
        }
        return;
    }

    let stacking_context = stacking_context_entry.or_default();
    if stacking_context.updated {
        trace!("Found root: {:?}, skipping", entity);
    } else {
        trace!("Found root: {:?}", entity);
        stacking_context.updated = true;
        stacking_context.context.clear();
        let starting_index = 0;
        stacking_context.root_entity = Some(entity);
        stacking_context.context.push((entity, starting_index));
        fill_stacking_context(entity, stacking_context, starting_index, style_query);
        stacking_context
            .context
            .sort_by_key(|(_entity, z_index)| *z_index);
    }
}

fn fill_stacking_context(
    entity: Entity,
    stacking_context: &mut StackingContext,
    current_index: i16,
    style_query: &Query<(
        &Style,
        Option<&Parent>,
        Option<&Children>,
        Option<&NodeWindowId>,
    )>,
) {
    if let Ok(children) = style_query.get_component::<Children>(entity) {
        for &child in children.iter() {
            if style_query.get_component::<NodeWindowId>(child).is_err() {
                if let Ok(style) = &style_query.get_component::<Style>(child) {
                    let z_index = style.z_index;
                    let new_index = z_index.get() + current_index;
                    stacking_context.context.push((child, new_index));
                    if z_index == ZIndex::Auto {
                        fill_stacking_context(child, stacking_context, new_index, style_query);
                    }
                }
            }
        }
    }
}

pub const UI_Z_STEP: f32 = 0.001;

fn update_transforms_z_impl(
    flex_surface: &FlexSurface,
    stacking_context: &StackingContext,
    current_z: &mut f32,
    transform_query: &mut Query<
        (&mut Transform, &mut GlobalTransform, Option<&Parent>),
        With<Style>,
    >,
) {
    for (entity, _z_index) in stacking_context.context.iter() {
        if let Some(child_stacking_context) = flex_surface
            .stacking_contexts
            .get(&entity)
            .filter(|_| stacking_context.root_entity != Some(*entity))
        {
            // This is not the root entity of this context, and it has its own context
            update_transforms_z_impl(
                flex_surface,
                child_stacking_context,
                current_z,
                transform_query,
            );
        } else if let Ok(mut global_transform) =
            transform_query.get_component_mut::<GlobalTransform>(*entity)
        {
            *current_z += UI_Z_STEP;
            global_transform.translation.z = *current_z;
        }
    }
    // This loop is not merged with the previous one since the "z-index order" can be different from the entity hierarchy order
    for (entity, _z_index) in stacking_context.context.iter() {
        if Some(*entity) != stacking_context.root_entity {
            let parent_global_z = transform_query
                .get_component::<Parent>(*entity)
                .and_then(|parent| transform_query.get_component::<GlobalTransform>(**parent))
                .map(|transform| transform.translation.z)
                .unwrap_or_default();
            if let Ok((mut transform, global_transform, _parent)) = transform_query.get_mut(*entity)
            {
                transform.translation.z = global_transform.translation.z - parent_global_z;
            }
        }
    }
}

pub fn garbage_collection_system(
    mut flex_surface: ResMut<FlexSurface>,
    style_query: Query<&Style>,
) {
    for entity in style_query.removed::<Style>() {
        if let Some(node) = flex_surface.entity_to_stretch.remove(entity) {
            trace!("Removing stretch node for: {:?}", entity);
            flex_surface.stretch.remove(node);
        }
        if flex_surface.stacking_contexts.remove(entity).is_some() {
            trace!("Removing stacking context for: {:?}", entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        Changed, Commands, Entity, IntoSystem, Or, Query, ResMut, Resources, Schedule, SystemStage,
        With, World,
    };
    use bevy_transform::{
        components::{Children, GlobalTransform, Parent, Transform},
        hierarchy::BuildChildren,
    };

    use super::{update_transforms_z, FlexSurface, Node, NodeWindowId, Style, ZIndex, UI_Z_STEP};

    fn node_with_transform(
        name: &str,
        z_index: ZIndex,
    ) -> (String, Node, Style, Transform, GlobalTransform) {
        (
            name.to_owned(),
            Node::default(),
            Style {
                z_index,
                ..Default::default()
            },
            Transform::default(),
            GlobalTransform::default(),
        )
    }

    fn node_without_transform(name: &str, z_index: ZIndex) -> (String, Node, Style) {
        (
            name.to_owned(),
            Node::default(),
            Style {
                z_index,
                ..Default::default()
            },
        )
    }

    fn get_steps(transform: &Transform) -> i32 {
        (transform.translation.z / UI_Z_STEP).round() as i32
    }

    pub fn update_transforms_z_system(
        mut flex_surface: ResMut<FlexSurface>,
        changed_zindex_query: Query<
            Entity,
            (
                With<Style>,
                Or<(Changed<Style>, Changed<Parent>, Changed<Children>)>,
            ),
        >,
        style_query: Query<(
            &Style,
            Option<&Parent>,
            Option<&Children>,
            Option<&NodeWindowId>,
        )>,
        mut transform_query: Query<
            (&mut Transform, &mut GlobalTransform, Option<&Parent>),
            With<Style>,
        >,
    ) {
        update_transforms_z(
            &mut *flex_surface,
            &changed_zindex_query,
            &style_query,
            &mut transform_query,
        );
    }

    #[test]
    fn test_ui_z_system() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

        commands.spawn(node_with_transform("0", ZIndex::Some(0)));
        let entity_0 = commands.current_entity().unwrap();

        commands
            .spawn(node_with_transform("1", ZIndex::Some(1)))
            .with_children(|parent| {
                parent
                    .spawn(node_with_transform("1-0", ZIndex::None))
                    .with_children(|parent| {
                        parent.spawn(node_without_transform("1-0-0", ZIndex::None));
                        parent.spawn(node_with_transform("1-0-1", ZIndex::None));
                        parent.spawn(node_with_transform("1-0-2", ZIndex::None));
                    });
                parent.spawn(node_with_transform("1-1", ZIndex::None));
                parent
                    .spawn(node_with_transform("1-2", ZIndex::None))
                    .with_children(|parent| {
                        parent.spawn(node_with_transform("1-2-0", ZIndex::None));
                        parent.spawn(node_with_transform("1-2-1", ZIndex::Some(-1)));
                        parent
                            .spawn(node_with_transform("1-2-2", ZIndex::None))
                            .with_children(|_| ());
                        parent.spawn(node_with_transform("1-2-3", ZIndex::None));
                    });
                parent.spawn(node_with_transform("1-3", ZIndex::None));
            });
        let entity_1 = commands.current_entity().unwrap();

        commands
            .spawn(node_with_transform("2", ZIndex::Some(2)))
            .with_children(|parent| {
                parent
                    .spawn(node_with_transform("2-0", ZIndex::None))
                    .with_children(|_parent| ());
                parent
                    .spawn(node_with_transform("2-1", ZIndex::Auto))
                    .with_children(|parent| {
                        parent.spawn(node_with_transform("2-1-0", ZIndex::Some(-1)));
                    });
            });
        let entity_2 = commands.current_entity().unwrap();
        commands.apply(&mut world, &mut resources);

        let mut flex_surface = FlexSurface::default();
        flex_surface.window_nodes.map.insert(
            entity_0,
            flex_surface
                .stretch
                .new_node(Default::default(), Default::default())
                .unwrap(),
        );
        flex_surface.window_nodes.map.insert(
            entity_1,
            flex_surface
                .stretch
                .new_node(Default::default(), Default::default())
                .unwrap(),
        );
        flex_surface.window_nodes.map.insert(
            entity_2,
            flex_surface
                .stretch
                .new_node(Default::default(), Default::default())
                .unwrap(),
        );
        flex_surface.window_nodes.dirty = true;
        resources.insert(flex_surface);

        let mut schedule = Schedule::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(update_transforms_z_system.system());
        schedule.add_stage("update", update_stage);
        schedule.initialize_and_run(&mut world, &mut resources);

        // GlobalTransform
        let mut actual_result = world
            .query::<(&String, &GlobalTransform)>()
            .map(|(name, &transform)| (name.clone(), get_steps(&transform.into())))
            .collect::<Vec<(String, i32)>>();
        actual_result.sort_unstable_by_key(|(name, _)| name.clone());
        let expected_result = vec![
            ("0".to_owned(), 1), // ZIndex::Some(0)
            ("1".to_owned(), 2), // ZIndex::Some(1)
            ("1-0".to_owned(), 3),
            // 1-0-0 has no transform
            ("1-0-1".to_owned(), 4),
            ("1-0-2".to_owned(), 5),
            ("1-1".to_owned(), 6),
            ("1-2".to_owned(), 8),
            ("1-2-0".to_owned(), 9),
            ("1-2-1".to_owned(), 7), // ZIndex::Some(-1), placed before 1-2
            ("1-2-2".to_owned(), 10),
            ("1-2-3".to_owned(), 11),
            ("1-3".to_owned(), 12),
            ("2".to_owned(), 14), // ZIndex::Some(2)
            ("2-0".to_owned(), 15),
            ("2-1".to_owned(), 16),   // ZIndex::Auto
            ("2-1-0".to_owned(), 13), // ZIndex::Some(-1), placed before 2
        ];
        assert_eq!(actual_result, expected_result);

        // Transform
        let mut actual_result = world
            .query::<(&String, &Transform)>()
            .map(|(name, transform)| (name.clone(), get_steps(transform)))
            .collect::<Vec<(String, i32)>>();
        actual_result.sort_unstable_by_key(|(name, _)| name.clone());
        let expected_result = vec![
            ("0".to_owned(), 1), // ZIndex::Some(0)
            ("1".to_owned(), 2), // ZIndex::Some(1)
            ("1-0".to_owned(), 1),
            // 1-0-0 has no transform
            ("1-0-1".to_owned(), 1),
            ("1-0-2".to_owned(), 2),
            ("1-1".to_owned(), 4),
            ("1-2".to_owned(), 6),
            ("1-2-0".to_owned(), 1),
            ("1-2-1".to_owned(), -1), // ZIndex::Some(-1), placed before 1-2
            ("1-2-2".to_owned(), 2),
            ("1-2-3".to_owned(), 3),
            ("1-3".to_owned(), 10),
            ("2".to_owned(), 14), // ZIndex::Some(2)
            ("2-0".to_owned(), 1),
            ("2-1".to_owned(), 2),    // ZIndex::Auto
            ("2-1-0".to_owned(), -3), // ZIndex::Some(-1), placed before 2
        ];
        assert_eq!(actual_result, expected_result);
    }
}
