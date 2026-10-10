use core::fmt::Write;

use bevy_ecs::{entity::Entity, world::World};

use crate::{layout::layout_tree::ComputedLayout, ContentSize, Display, Node, UiRoots};

/// Prints the latest computed UI layout tree for each root node.
pub fn print_ui_layout_tree(world: &World) {
    let Some(ui_roots) = world.get_resource::<UiRoots>() else {
        return;
    };
    for entity in ui_roots.layout_roots() {
        let mut out = String::new();
        print_node(world, entity, false, String::new(), &mut out);

        tracing::info!("Layout tree for root entity: {entity}\n{out}");
    }
}

/// Recursively navigates the layout tree printing each node's information.
fn print_node(
    world: &World,
    entity: Entity,
    has_sibling: bool,
    lines_string: String,
    acc: &mut String,
) {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return;
    };
    let Ok((node, computed_layout, content_size)) =
        entity_ref.get_components::<(&Node, &ComputedLayout, &ContentSize)>()
    else {
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
        print_node(world, child_entity, has_sibling, new_string.clone(), acc);
    }
}
