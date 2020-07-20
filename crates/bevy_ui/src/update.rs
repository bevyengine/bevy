use super::Node;
use bevy_ecs::{Entity, Query, Res, Without};
use bevy_math::Vec2;
use bevy_transform::{
    hierarchy,
    prelude::{Children, LocalTransform, Parent},
};
use bevy_window::Windows;

pub const UI_Z_STEP: f32 = 0.001;

#[derive(Clone)]
pub struct Rect {
    pub z: f32,
    pub size: Vec2,
}

pub fn ui_update_system(
    windows: Res<Windows>,
    mut orphan_node_query: Query<Without<Parent, (Entity, &mut Node, &mut LocalTransform)>>,
    mut node_query: Query<(Entity, &mut Node, &mut LocalTransform)>,
    children_query: Query<&Children>,
) {
    let window_size = if let Some(window) = windows.get_primary() {
        Vec2::new(window.width as f32, window.height as f32)
    } else {
        return;
    };
    let orphan_nodes = orphan_node_query
        .iter()
        .iter()
        .map(|(e, _, _)| e)
        .collect::<Vec<Entity>>();
    let mut window_rect = Rect {
        z: 0.0,
        size: window_size,
    };

    let mut previous_sibling_result = Some(Rect {
        z: 0.0,
        size: window_size,
    });
    for entity in orphan_nodes {
        previous_sibling_result = hierarchy::run_on_hierarchy(
            &children_query,
            &mut node_query,
            entity,
            Some(&mut window_rect),
            previous_sibling_result,
            &mut update_node_entity,
        );
    }
}

fn update_node_entity(
    node_query: &mut Query<(Entity, &mut Node, &mut LocalTransform)>,
    entity: Entity,
    parent_rect: Option<&mut Rect>,
    previous_rect: Option<Rect>,
) -> Option<Rect> {
    if let Ok(mut node) = node_query.get_mut::<Node>(entity) {
        if let Ok(mut local_transform) = node_query.get_mut::<LocalTransform>(entity) {
            let parent_rect = parent_rect.unwrap();
            let mut z = UI_Z_STEP;
            if let Some(previous_rect) = previous_rect {
                z += previous_rect.z
            };

            node.update(&mut local_transform, z, parent_rect.size);
            return Some(Rect { size: node.size, z });
        }
    }

    None
}
