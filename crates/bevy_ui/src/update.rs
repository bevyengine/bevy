use super::Node;
use crate::FlexSurfaceId;
use bevy_ecs::{Entity, Query, With, Without};
use bevy_transform::{
    hierarchy,
    prelude::{Children, LocalTransform, Parent},
};

pub const UI_Z_STEP: f32 = 0.001;

pub fn ui_z_system(
    mut root_node_query: Query<With<Node, Without<Parent, (Entity, &FlexSurfaceId)>>>,
    mut node_query: Query<(Entity, &Node, &mut FlexSurfaceId, &mut LocalTransform)>,
    children_query: Query<&Children>,
) {
    let mut window_z = 0.0;

    // PERF: we can probably avoid an allocation here by making root_node_query and node_query non-overlapping
    let root_nodes = (&mut root_node_query.iter())
        .iter()
        .map(|(e, s)| (e, *s))
        .collect::<Vec<(Entity, FlexSurfaceId)>>();

    for (entity, flex_surface_id) in root_nodes {
        if let Some(result) = hierarchy::run_on_hierarchy(
            &children_query,
            &mut node_query,
            entity,
            Some((flex_surface_id, window_z)),
            Some((flex_surface_id, window_z)),
            &mut update_node_entity,
        ) {
            window_z = result.1;
        }
    }
}

fn update_node_entity(
    node_query: &mut Query<(Entity, &Node, &mut FlexSurfaceId, &mut LocalTransform)>,
    entity: Entity,
    parent_result: Option<(FlexSurfaceId, f32)>,
    previous_result: Option<(FlexSurfaceId, f32)>,
) -> Option<(FlexSurfaceId, f32)> {
    let mut surface_id = node_query.get_mut::<FlexSurfaceId>(entity).unwrap();
    let mut transform = node_query.get_mut::<LocalTransform>(entity).unwrap();
    let (parent_surface_id, _) = parent_result?;
    let mut z = UI_Z_STEP;
    if let Some((_, previous_z)) = previous_result {
        z += previous_z;
    };

    let mut position = transform.w_axis();
    position.set_z(z);
    transform.set_w_axis(position);

    *surface_id = parent_surface_id;

    return Some((parent_surface_id, z));
}
