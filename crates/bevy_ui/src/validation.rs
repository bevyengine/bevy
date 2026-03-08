use crate::{
    widget::{ImageNode, Text, ViewportNode},
    Node,
};
use bevy_ecs::prelude::*;
use tracing::warn;

fn mixed_leaf_content_components(
    has_text: bool,
    has_image_node: bool,
    has_viewport_node: bool,
) -> Vec<&'static str> {
    let mut components = Vec::with_capacity(3);
    if has_text {
        components.push("Text");
    }
    if has_image_node {
        components.push("ImageNode");
    }
    if has_viewport_node {
        components.push("ViewportNode");
    }
    components
}

/// Warns when an entity mixes multiple leaf UI content components on the same node.
///
/// A single UI node should use one leaf content component (`Text`, `ImageNode`, or `ViewportNode`).
/// To combine multiple content types, use a parent `Node` and put each leaf on separate children.
pub fn warn_on_invalid_mixed_leaf_content(
    query: Query<
        (Entity, Has<Text>, Has<ImageNode>, Has<ViewportNode>),
        (
            With<Node>,
            Or<(Added<Text>, Added<ImageNode>, Added<ViewportNode>)>,
        ),
    >,
) {
    for (entity, has_text, has_image_node, has_viewport_node) in &query {
        let mixed_components =
            mixed_leaf_content_components(has_text, has_image_node, has_viewport_node);
        if mixed_components.len() <= 1 {
            continue;
        }

        warn!(
            "UI entity {entity} mixes multiple leaf content components ({mixed_components}). \
Use one leaf component per entity and compose with child nodes.",
            mixed_components = mixed_components.join(", ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::mixed_leaf_content_components;
    use crate::{
        widget::{ImageNode, Text, ViewportNode},
        Node,
    };
    use bevy_ecs::{entity::Entity, prelude::*, world::World};

    fn collect_invalid_leaf_content(world: &mut World) -> Vec<(Entity, Vec<&'static str>)> {
        let mut query = world.query::<(Entity, Has<Text>, Has<ImageNode>, Has<ViewportNode>)>();
        query
            .iter(world)
            .filter_map(|(entity, has_text, has_image_node, has_viewport_node)| {
                let mixed =
                    mixed_leaf_content_components(has_text, has_image_node, has_viewport_node);
                (mixed.len() > 1).then_some((entity, mixed))
            })
            .collect()
    }

    #[test]
    fn mixed_leaf_content_detects_invalid_combinations() {
        let mut world = World::new();
        world.spawn((Text::new("hello"), ImageNode::default()));
        let camera = world.spawn_empty().id();
        world.spawn((Text::new("viewport"), ViewportNode::new(camera)));

        let invalid = collect_invalid_leaf_content(&mut world);
        assert_eq!(invalid.len(), 2);
        assert_eq!(invalid[0].1, vec!["Text", "ImageNode"]);
        assert_eq!(invalid[1].1, vec!["Text", "ViewportNode"]);
    }

    #[test]
    fn parent_child_composition_is_not_flagged() {
        let mut world = World::new();
        let parent = world.spawn(Node::default()).id();
        let text = world.spawn(Text::new("text child")).id();
        let image = world.spawn(ImageNode::default()).id();
        world.entity_mut(parent).add_children(&[text, image]);

        let invalid = collect_invalid_leaf_content(&mut world);
        assert!(invalid.is_empty());
    }
}
