use super::Node;
use bevy_core::transform::run_on_hierarchy_subworld_mut;
use bevy_transform::prelude::{Children, Parent};
use bevy_window::Windows;
use glam::Vec2;
use legion::{prelude::*, systems::SubWorld};
use bevy_sprite::Rect;

pub const UI_Z_STEP: f32 = 0.0001;

pub fn ui_update_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("ui_update")
        .read_resource::<Windows>()
        .with_query(<Read<Node>>::query().filter(!component::<Parent>()))
        .write_component::<Node>()
        .write_component::<Rect>()
        .read_component::<Children>()
        .build(move |_, world, windows, node_query| {
            if let Some(window) = windows.get_primary() {
                let mut window_rect = Rect {
                    size: Vec2::new(window.width as f32, window.height as f32),
                    position: Vec2::new(0.0, 0.0),
                    z_index: 0.9999,
                };
                for entity in node_query
                    .iter_entities(world)
                    .map(|(e, _)| e)
                    .collect::<Vec<Entity>>()
                {
                    let result = run_on_hierarchy_subworld_mut(
                        world,
                        entity,
                        window_rect.clone(),
                        &mut update_node_entity,
                        &mut process_child_result,
                    );

                    if let Some(result) = result {
                        window_rect.z_index = result.z_index;
                    }
                }
            }
        })
}

fn update_node_entity(world: &mut SubWorld, entity: Entity, parent_rect: Rect) -> Option<Rect> {
    // TODO: Somehow remove this unsafe
    unsafe {
        if let Some(mut node) = world.get_component_mut_unchecked::<Node>(entity) {
            if let Some(mut rect) = world.get_component_mut_unchecked::<Rect>(entity) {
                node.update(
                    &mut rect,
                    parent_rect.size,
                    parent_rect.position,
                    parent_rect.z_index,
                );
                return Some(Rect {
                    size: rect.size,
                    position: rect.position - rect.size / 2.0,
                    z_index: rect.z_index - UI_Z_STEP,
                });
            }
        }
    }

    None
}

fn process_child_result(_parent_result: Rect, child_result: Rect) -> Rect {
    // "earlier" children are sorted behind "later" children
    let mut result = child_result.clone();
    result.z_index -= UI_Z_STEP;
    result
}
