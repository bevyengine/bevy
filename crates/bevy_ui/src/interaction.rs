use crate::Node;
use bevy_app::{EventReader, Events};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButton, Input};
use bevy_math::{vec2, Vec2};
use bevy_transform::components::Transform;
use bevy_window::CursorMoved;
use std::collections::HashSet;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PropagatePolicy {
    Block,
    Pass,
}

impl Default for PropagatePolicy {
    fn default() -> Self {
        PropagatePolicy::Block
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Interaction {
    Pressed,
    Hovered,
    None,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum RegionAction {
    Enter,
    Exit,
    Hover(Vec2),
    Move(Vec2),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PressAction {
    Up,
    Down,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PointerRegion {
    pub entity: Entity,
    pub action: RegionAction,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct PointerPress {
    pub entity: Entity,
    pub action: PressAction,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct PointerClick {
    pub entity: Entity,
}

pub struct EventState {
    cursor_moved_event_reader: EventReader<CursorMoved>,
    cursor_position: Vec2,
    hovered_entities: HashSet<Entity>,
    pressed_entities: HashSet<Entity>,
}

impl Default for EventState {
    fn default() -> Self {
        EventState {
            cursor_moved_event_reader: Default::default(),
            cursor_position: Default::default(),
            hovered_entities: HashSet::new(),
            pressed_entities: HashSet::new(),
        }
    }
}

pub fn ui_event_system(
    mut state: Local<EventState>,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mut region_events: ResMut<Events<PointerRegion>>,
    mut click_events: ResMut<Events<PointerClick>>,
    mut press_events: ResMut<Events<PointerPress>>,
    mut node_query: Query<(
        Entity,
        &Node,
        &mut Interaction,
        &Transform,
        Option<&PropagatePolicy>,
    )>,
) {
    let mut cursor_has_moved = false;
    if let Some(cursor_moved) = state.cursor_moved_event_reader.latest(&cursor_moved_events) {
        state.cursor_position = cursor_moved.position;
        cursor_has_moved = true;
    }

    let mut new_hovered_entities = HashSet::new();
    {
        let mut query_iter = node_query.iter();
        let mut moused_over_z_sorted_nodes = query_iter
            .iter()
            .filter_map(|(entity, node, interaction, transform, propagate_policy)| {
                let position = transform.value.w_axis();
                let ui_position = position.truncate().truncate();
                let extents = node.size / 2.0;
                let min = ui_position - extents;
                let max = ui_position + extents;
                let cursor_x = state.cursor_position.x();
                let cursor_y = state.cursor_position.y();

                // if the current cursor position is within the bounds of the node, consider it for events
                if (min.x()..max.x()).contains(&cursor_x) && (min.y()..max.y()).contains(&cursor_y)
                {
                    Some((
                        entity,
                        interaction,
                        vec2(cursor_x - min.x(), cursor_y - min.y()),
                        propagate_policy,
                        FloatOrd(position.z()),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        moused_over_z_sorted_nodes.sort_by_key(|(_, _, _, _, z)| -*z);

        let mouse_pressed = mouse_button_input.just_pressed(MouseButton::Left);
        let mouse_released = mouse_button_input.just_released(MouseButton::Left);

        for (entity, mut interaction, position, propagate_policy, _) in moused_over_z_sorted_nodes {
            new_hovered_entities.insert(entity);

            if !state.hovered_entities.contains(&entity) {
                state.hovered_entities.insert(entity);

                region_events.send(PointerRegion {
                    entity,
                    action: RegionAction::Enter,
                });
                if state.pressed_entities.contains(&entity) {
                    *interaction = Interaction::Pressed;
                } else {
                    *interaction = Interaction::Hovered;
                }
            }

            if mouse_pressed {
                state.pressed_entities.insert(entity);

                press_events.send(PointerPress {
                    entity,
                    action: PressAction::Down,
                });

                *interaction = Interaction::Pressed;
            }

            if mouse_released {
                press_events.send(PointerPress {
                    entity,
                    action: PressAction::Up,
                });
                if state.pressed_entities.contains(&entity) {
                    click_events.send(PointerClick { entity });
                }

                *interaction = Interaction::Hovered;
                state.pressed_entities.clear();
            }

            if cursor_has_moved {
                if state.pressed_entities.contains(&entity) {
                    region_events.send(PointerRegion {
                        entity,
                        action: RegionAction::Move(position),
                    });
                } else {
                    region_events.send(PointerRegion {
                        entity,
                        action: RegionAction::Hover(position),
                    });
                }
            }

            match propagate_policy.cloned().unwrap_or(PropagatePolicy::Block) {
                PropagatePolicy::Block => {
                    break;
                }
                PropagatePolicy::Pass => { /* allow the next node to be hovered/clicked */ }
            }
        }
    }

    for entity in state
        .hovered_entities
        .clone()
        .difference(&new_hovered_entities)
    {
        region_events.send(PointerRegion {
            entity: *entity,
            action: RegionAction::Exit,
        });

        if let Ok(mut interaction) = node_query.get_mut::<Interaction>(*entity) {
            *interaction = Interaction::None;
        }
    }

    state.hovered_entities = new_hovered_entities;
}
