use core::fmt::Write;

use bevy_ecs::{entity::Entity, hierarchy::ChildOf, query::With, system::Query};

use crate::{
    experimental::{UiChildren, UiRootNodes},
    layout::layout_tree::ComputedLayout,
    ContentSize, Display, FixedNode, Node,
};

/// Prints the latest computed UI layout tree for each root node.
pub fn print_ui_layout_tree(
    root_node_query: UiRootNodes,
    fixed_nodes_query: Query<Entity, (With<FixedNode>, With<ChildOf>)>,
    ui_children: UiChildren,
    layout_query: Query<(&Node, &ComputedLayout, &ContentSize)>,
) {
    for entity in root_node_query.iter().chain(fixed_nodes_query.iter()) {
        let mut out = String::new();
        print_node(
            &ui_children,
            &fixed_nodes_query,
            &layout_query,
            entity,
            false,
            String::new(),
            &mut out,
        );

        tracing::info!("Layout tree for root entity: {entity}\n{out}");
    }
}

/// Recursively navigates the layout tree printing each node's information.
fn print_node(
    ui_children: &UiChildren,
    fixed_nodes_query: &Query<Entity, (With<FixedNode>, With<ChildOf>)>,
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

    let num_children = ui_children
        .iter_ui_children(entity)
        .filter(|child| !fixed_nodes_query.contains(*child))
        .count();

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
    for (index, child_entity) in ui_children
        .iter_ui_children(entity)
        .filter(|child| !fixed_nodes_query.contains(*child))
        .enumerate()
    {
        let has_sibling = index < num_children - 1;
        print_node(
            ui_children,
            fixed_nodes_query,
            layout_query,
            child_entity,
            has_sibling,
            new_string.clone(),
            acc,
        );
    }
}
