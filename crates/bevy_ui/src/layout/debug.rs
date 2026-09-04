use core::fmt::Write;

use bevy_ecs::{
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Has, With, Without},
    system::{Local, Query},
    world::Ref,
};

use crate::{
    layout::{
        layout_tree::{collect_ui_children, entity_node_id, node_id_entity, ComputedLayout},
        UiTreeChanged,
    },
    ContentSize, Display, FixedNode, GhostNode, Node,
};

/// Prints the latest computed UI layout tree for each root node.
pub fn print_ui_layout_tree(
    root_node_query: Query<Entity, (With<Node>, Without<ChildOf>)>,
    fixed_nodes_query: Query<(Entity, Has<GhostNode>), (With<FixedNode>, With<ChildOf>)>,
    ui_hierarchy: Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeChanged>), With<Node>>,
    layout_query: Query<(&Node, &ComputedLayout, &ContentSize)>,
    mut root_stack: Local<Vec<taffy::NodeId>>,
) {
    root_stack.clear();
    for entity in root_node_query.iter() {
        if ui_hierarchy
            .get(entity)
            .is_ok_and(|(_, is_ghost, _)| is_ghost)
        {
            collect_ui_children(entity, &ui_hierarchy, &mut root_stack, &mut vec![]);
        } else {
            root_stack.push(entity_node_id(entity));
        }
    }
    root_stack.retain(|node_id| !fixed_nodes_query.contains(node_id_entity(*node_id)));
    root_stack.extend(
        fixed_nodes_query
            .iter()
            .filter_map(|(entity, is_ghost)| (!is_ghost).then_some(entity_node_id(entity))),
    );

    for entity in root_stack.iter().copied().map(node_id_entity) {
        let mut out = String::new();
        print_node(&layout_query, entity, false, String::new(), &mut out);

        tracing::info!("Layout tree for root entity: {entity}\n{out}");
    }
}

/// Recursively navigates the layout tree printing each node's information.
fn print_node(
    layout_query: &Query<(&Node, &ComputedLayout, &ContentSize)>,
    entity: Entity,
    has_sibling: bool,
    lines_string: String,
    acc: &mut String,
) {
    let Ok((node, computed_layout, content_size)) = layout_query.get(entity) else {
        return;
    };
    let Some((layout, _)) = computed_layout.get_layout(true) else {
        return;
    };

    let num_children = computed_layout.child_nodes().len();

    let display_variant = match (num_children, node.display) {
        (_, Display::None) => "NONE",
        (0, _) => "LEAF",
        (_, Display::Flex) => "FLEX",
        (_, Display::Grid) => "GRID",
        (_, Display::Block) => "BLOCK",
    };

    let fork_string = if has_sibling {
        "├── "
    } else {
        "└── "
    };
    writeln!(
        acc,
        "{lines}{fork} {display} [x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({entity}) {measured}",
        lines = lines_string,
        fork = fork_string,
        display = display_variant,
        x = layout.location.x,
        y = layout.location.y,
        width = layout.size.width,
        height = layout.size.height,
        measured = if content_size.measure.is_some() {
            "measured"
        } else {
            ""
        }
    )
    .ok();
    let bar = if has_sibling { "│   " } else { "    " };
    let new_string = lines_string + bar;

    // Recurse into children
    for (index, child_entity) in computed_layout.child_entities().enumerate() {
        let has_sibling = index < num_children - 1;
        print_node(
            layout_query,
            child_entity,
            has_sibling,
            new_string.clone(),
            acc,
        );
    }
}
