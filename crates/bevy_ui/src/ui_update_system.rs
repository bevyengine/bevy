use super::Node;
use bevy_core::transform::run_on_hierarchy_subworld_mut;
use bevy_transform::prelude::{Children, Parent};
use bevy_window::Windows;
use glam::Vec2;
use legion::{prelude::*, systems::SubWorld};

pub fn ui_update_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("ui_update")
        .read_resource::<Windows>()
        .with_query(<(Write<Node>,)>::query().filter(!component::<Parent>()))
        .write_component::<Node>()
        .read_component::<Children>()
        .build(move |_, world, windows, node_query| {
            if let Some(window) = windows.get_primary() {
                let parent_size = glam::vec2(window.width as f32, window.height as f32);
                let parent_position = glam::vec2(0.0, 0.0);
                for (entity, _) in node_query.iter_entities_mut(world) {
                    run_on_hierarchy_subworld_mut(
                        world,
                        entity,
                        (parent_size, parent_position),
                        &mut update_node_entity,
                    );
                }
            }
        })
}

fn update_node_entity(
    world: &mut SubWorld,
    entity: Entity,
    parent_properties: (Vec2, Vec2),
) -> Option<(Vec2, Vec2)> {
    let (parent_size, parent_position) = parent_properties;
    if let Some(mut node) = world.get_component_mut::<Node>(entity) {
        node.update(parent_size, parent_position);
        return Some((node.size, node.global_position));
    }

    None
}
