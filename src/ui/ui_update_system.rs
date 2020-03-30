use crate::prelude::*;

pub fn ui_update_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("ui_update")
        .read_resource::<Window>()
        .with_query(<(Write<Node>,)>::query().filter(!component::<Parent>()))
        .write_component::<Node>()
        .read_component::<Children>()
        .build(move |_, world, window, node_query| {
            let parent_size = math::vec2(window.width as f32, window.height as f32);
            let parent_position = math::vec2(0.0, 0.0);
            for (entity, _) in node_query.iter_entities_mut(world) {
                ecs::run_on_hierarchy_subworld_mut(
                    world,
                    entity,
                    (parent_size, parent_position),
                    &mut update_node_entity,
                );
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
