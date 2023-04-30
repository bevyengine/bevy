use bevy_ecs::prelude::Entity;
use bevy_utils::HashMap;
use taffy::prelude::Node;
use taffy::tree::LayoutTree;
use crate::UiSurface;

pub fn print_ui_layout_tree(ui_surface: &UiSurface) {
    let taffy_to_entity: HashMap<Node, Entity> =
        ui_surface.entity_to_taffy.iter()
        .map(|(entity, node)| (*node, *entity))
        .collect();
    for (&entity, &node) in ui_surface.window_nodes.iter() {
        println!("Layout tree for window entity: {entity:?}");
        print_node(ui_surface, &taffy_to_entity, entity, node, false, String::new());
    }
}

fn print_node(ui_surface: &UiSurface, taffy_to_entity: &HashMap<Node, Entity>, entity: Entity, node: Node, has_sibling: bool, lines_string: String) {
    let tree = &ui_surface.taffy;
    let layout = tree.layout(node).unwrap();
    let style = tree.style(node).unwrap();

    let num_children = tree.child_count(node).unwrap();

    let display = match (num_children, style.display) {
        (_, taffy::style::Display::None) => "NONE",
        (0, _) => "LEAF",
        (_, taffy::style::Display::Flex) => "FLEX",
        (_, taffy::style::Display::Grid) => "GRID",
    };

    let fork_string = if has_sibling { "├── " } else { "└── " };
    println!(
        "{lines}{fork} {display} [x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({entity:?}) {measured}",
        lines = lines_string,
        fork = fork_string,
        display = display,
        x = layout.location.x,
        y = layout.location.y,
        width = layout.size.width,
        height = layout.size.height,
        measured = if tree.needs_measure(node) { "measured" } else { "" }
    );
    let bar = if has_sibling { "│   " } else { "    " };
    let new_string = lines_string + bar;

    // Recurse into children
    for (index, child_node) in tree.children(node).unwrap().iter().enumerate() {
        let has_sibling = index < num_children - 1;
        let child_entity = taffy_to_entity.get(child_node).unwrap();
        print_node(ui_surface, &taffy_to_entity, *child_entity, *child_node, has_sibling, new_string.clone());
    }
}