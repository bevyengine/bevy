use super::Node;
use bevy_core::transform::run_on_hierarchy_subworld_mut;
use bevy_transform::prelude::{Children, Parent, Translation};
use bevy_window::Windows;
use glam::Vec2;
use legion::{prelude::*, systems::SubWorld};

pub const UI_Z_STEP: f32 = 0.001;

#[derive(Clone)]
pub struct Rect {
    pub z: f32,
    pub size: Vec2,
}

pub fn ui_update_system(
    windows: Res<Windows>,
    world: &mut SubWorld,
    node_query: &mut Query<(Write<Node>, Write<Translation>)>,
    _parent_query: &mut Query<Read<Parent>>,
    _children_query: &mut Query<Read<Children>>,
) {
    let window_size = if let Some(window) = windows.get_primary() {
        Vec2::new(window.width as f32, window.height as f32)
    } else {
        return;
    };
    let orphan_nodes = node_query
        .iter_entities_mut(world)
        // TODO: replace this filter with a legion query filter (when SimpleQuery gets support for filters)
        .filter(|(entity, _)| world.get_component::<Parent>(*entity).is_none())
        .map(|(e, _)| e)
        .collect::<Vec<Entity>>();
    let mut window_rect = Rect {
        z: 0.0,
        size: window_size,
    };

    let mut previous_sibling_result = Some(Rect {
        z: 999.0,
        size: window_size,
    });
    for entity in orphan_nodes {
        previous_sibling_result = run_on_hierarchy_subworld_mut(
            world,
            entity,
            Some(&mut window_rect),
            previous_sibling_result,
            &mut update_node_entity,
        );
    }
}

fn update_node_entity(
    world: &mut SubWorld,
    entity: Entity,
    parent_rect: Option<&mut Rect>,
    previous_rect: Option<Rect>,
) -> Option<Rect> {
    // TODO: Somehow remove this unsafe
    unsafe {
        if let Some(mut node) = world.get_component_mut_unchecked::<Node>(entity) {
            if let Some(mut translation) = world.get_component_mut_unchecked::<Translation>(entity)
            {
                let parent_rect = parent_rect.unwrap();
                let mut z = parent_rect.z;
                if let Some(previous_rect) = previous_rect {
                    z = previous_rect.z
                };

                z -= UI_Z_STEP;
                node.update(&mut translation, z - parent_rect.z, parent_rect.size);
                return Some(Rect { size: node.size, z });
            }
        }
    }

    None
}
