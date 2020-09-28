use super::Node;
use bevy_ecs::{Entity, Query, With, Without};
use bevy_transform::{
    hierarchy,
    prelude::{Children, Parent, Transform},
};

pub const UI_Z_STEP: f32 = 0.001;

pub fn ui_z_system(
    mut root_node_query: Query<With<Node, Without<Parent, Entity>>>,
    mut node_query: Query<(Entity, &Node, &mut Transform)>,
    children_query: Query<&Children>,
) {
    let mut current_global_z = 0.0;

    // PERF: we can probably avoid an allocation here by making root_node_query and node_query non-overlapping
    let root_nodes = (&mut root_node_query.iter())
        .iter()
        .collect::<Vec<Entity>>();

    for entity in root_nodes {
        if let Some(result) = hierarchy::run_on_hierarchy(
            &children_query,
            &mut node_query,
            entity,
            Some(current_global_z),
            Some(current_global_z),
            &mut update_node_entity,
        ) {
            current_global_z = result;
        }
    }
}

fn update_node_entity(
    node_query: &mut Query<(Entity, &Node, &mut Transform)>,
    entity: Entity,
    parent_result: Option<f32>,
    previous_result: Option<f32>,
) -> Option<f32> {
    let mut z = UI_Z_STEP;
    let parent_global_z = parent_result.unwrap();
    if let Some(previous_global_z) = previous_result {
        z += previous_global_z - parent_global_z;
    };
    let global_z = z + parent_global_z;

    let mut transform = node_query.get_mut::<Transform>(entity).ok()?;
    transform.translation_mut().set_z(z);

    Some(global_z)
}
