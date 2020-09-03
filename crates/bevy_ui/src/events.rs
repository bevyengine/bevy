use crate::Node;
use bevy_app::{EventReader, Events};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButton, Input};
use bevy_math::Vec2;
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

#[derive(Debug, Clone)]
pub struct MouseDown {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct MouseUp {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct MouseEnter {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct MouseLeave {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct MouseHover {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct Click {
    pub entity: Entity,
}

#[derive(Debug, Clone)]
pub struct DoubleClick {
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
    mut events: (
        ResMut<Events<MouseDown>>,
        ResMut<Events<MouseUp>>,
        ResMut<Events<MouseEnter>>,
        ResMut<Events<MouseLeave>>,
        ResMut<Events<MouseHover>>,
        ResMut<Events<Click>>,
        ResMut<Events<DoubleClick>>,
    ),
    mut node_query: Query<(Entity, &Node, &Transform, Option<&PropagatePolicy>)>,
) {
    let (
        mut mouse_down_events,
        mut mouse_up_events,
        mut mouse_enter_events,
        mut mouse_leave_events,
        mut mouse_hover_events,
        mut click_events,
        mut double_click_events,
    ) = events;

    if let Some(cursor_moved) = state.cursor_moved_event_reader.latest(&cursor_moved_events) {
        state.cursor_position = cursor_moved.position;
    }

    let mut query_iter = node_query.iter();
    let mut moused_over_z_sorted_nodes = query_iter
        .iter()
        .filter_map(|(entity, node, transform, propagate_policy)| {
            let position = transform.value.w_axis();
            let ui_position = position.truncate().truncate();
            let extents = node.size / 2.0;
            let min = ui_position - extents;
            let max = ui_position + extents;
            // if the current cursor position is within the bounds of the node, consider it for events
            if (min.x()..max.x()).contains(&state.cursor_position.x())
                && (min.y()..max.y()).contains(&state.cursor_position.y())
            {
                Some((entity, propagate_policy, FloatOrd(position.z())))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    moused_over_z_sorted_nodes.sort_by_key(|(_, _, z)| -*z);

    let mouse_pressed = mouse_button_input.just_pressed(MouseButton::Left);
    let mouse_released = mouse_button_input.just_released(MouseButton::Left);

    let mut new_hovered_entities = HashSet::new();
    for (entity, propagate_policy, _) in moused_over_z_sorted_nodes {
        new_hovered_entities.insert(entity);
        if !state.hovered_entities.contains(&entity) {
            mouse_enter_events.send(MouseEnter { entity });
            state.hovered_entities.insert(entity);
        }

        if mouse_pressed {
            state.pressed_entities.insert(entity);
            mouse_down_events.send(MouseDown { entity });
        }
        if mouse_released {
            mouse_up_events.send(MouseUp { entity });
            if state.pressed_entities.contains(&entity) {
                click_events.send(Click { entity });
            }
        }

        mouse_hover_events.send(MouseHover { entity });

        match propagate_policy.cloned().unwrap_or(PropagatePolicy::Block) {
            PropagatePolicy::Block => {
                break;
            }
            PropagatePolicy::Pass => { /* allow the next node to be hovered/clicked */ }
        }
    }

    let mut unhovered_entities = HashSet::new();
    for entity in &state.hovered_entities {
        if !new_hovered_entities.contains(&entity) {
            unhovered_entities.insert(entity.clone());
        }
    }

    for entity in unhovered_entities {
        mouse_leave_events.send(MouseLeave {
            entity: entity.clone(),
        });
        if state.pressed_entities.contains(&entity) {
            state.pressed_entities.remove(&entity);
        }
    }

    state.hovered_entities = new_hovered_entities;
}
