
pub fn print_tree(tree: &impl LayoutTree, root: Node) {
    println!("TREE");
    print_node(tree, root, false, String::new());
}

fn print_node(tree: &impl LayoutTree, node: Node, has_sibling: bool, lines_string: String) {
    let layout = tree.layout(node);
    let style = tree.style(node);

    let num_children = tree.child_count(node);

    let display = match (num_children, style.display) {
        (_, style::Display::None) => "NONE",
        (0, _) => "LEAF",
        (_, style::Display::Flex) => "FLEX",
        #[cfg(feature = "grid")]
        (_, style::Display::Grid) => "GRID",
    };

    let fork_string = if has_sibling { "├── " } else { "└── " };
    println!(
        "{lines}{fork} {display} [x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({key:?})",
        lines = lines_string,
        fork = fork_string,
        display = display,
        x = layout.location.x,
        y = layout.location.y,
        width = layout.size.width,
        height = layout.size.height,
        key = node.data(),
    );
    let bar = if has_sibling { "│   " } else { "    " };
    let new_string = lines_string + bar;

    // Recurse into children
    for (index, child) in tree.children(node).enumerate() {
        let has_sibling = index < num_children - 1;
        print_node(tree, *child, has_sibling, new_string.clone());
    }
}