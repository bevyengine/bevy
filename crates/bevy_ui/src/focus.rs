use crate::Node;
use bevy_app::{EventReader, Events};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButton, Input};
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_window::CursorMoved;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Interaction {
    Clicked(MouseFlags),
    Hovered,
    None,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct MouseFlags(u16);

impl MouseFlags {
    pub fn check(self, button: MouseButton) -> bool {
        let i: u16 = button.into();
        ((self.0 << i) >> 15) > 0
    }

    pub fn build(button: &MouseButton) -> Self {
        button.clone().into()
    }

    pub fn remove(&mut self, button: &MouseButton) {
        let i: u16 = button.clone().into();
        let mask = !((u16::MAX << 15) >> i);
        self.0 &= mask;
    }

    pub fn add(&mut self, button: &MouseButton) {
        let i: u16 = button.clone().into();
        let mask = (u16::MAX << 15) >> i;
        self.0 |= mask;
    }
}

impl From<MouseButton> for MouseFlags {
    fn from(button: MouseButton) -> Self {
        let i: u16 = button.into();
        MouseFlags((u16::MAX << 15) >> i)
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FocusPolicy {
    Block,
    Pass,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        FocusPolicy::Block
    }
}

#[derive(Default)]
pub struct State {
    cursor_moved_event_reader: EventReader<CursorMoved>,
    cursor_position: Vec2,
    hovered_entity: Option<Entity>,
}

pub fn ui_focus_system(
    mut state: Local<State>,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mut node_query: Query<(
        Entity,
        &Node,
        &Transform,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
    )>,
) {
    if let Some(cursor_moved) = state.cursor_moved_event_reader.latest(&cursor_moved_events) {
        state.cursor_position = cursor_moved.position;
    }

    for (_entity, _node, _transform, interaction, _focus_policy) in &mut node_query.iter() {
        if let Some(mut interaction) = interaction {
            for released in mouse_button_input.get_just_released() {
                if let Interaction::Clicked(mut flags) = *interaction {
                    flags.remove(released);
                    *interaction = Interaction::Clicked(flags);
                }
            }

            if let Interaction::Clicked(MouseFlags(0)) = *interaction {
                *interaction = Interaction::None;
            }
        }
    }

    let mut hovered_entity = None;

    {
        let mut query_iter = node_query.iter();
        let mut moused_over_z_sorted_nodes = query_iter
            .iter()
            .filter_map(|(entity, node, transform, interaction, focus_policy)| {
                let position = transform.value.w_axis();
                let ui_position = position.truncate().truncate();
                let extents = node.size / 2.0;
                let min = ui_position - extents;
                let max = ui_position + extents;
                // if the current cursor position is within the bounds of the node, consider it for clicking
                if (min.x()..max.x()).contains(&state.cursor_position.x())
                    && (min.y()..max.y()).contains(&state.cursor_position.y())
                {
                    Some((entity, focus_policy, interaction, FloatOrd(position.z())))
                } else {
                    if let Some(mut interaction) = interaction {
                        if *interaction == Interaction::Hovered {
                            *interaction = Interaction::None;
                        }
                    }
                    None
                }
            })
            .collect::<Vec<_>>();

        moused_over_z_sorted_nodes.sort_by_key(|(_, _, _, z)| -*z);
        for (entity, focus_policy, interaction, _) in moused_over_z_sorted_nodes {
            if let Some(mut interaction) = interaction {
                // Build bitflags out of hashset
                let new_flags = mouse_button_input.get_just_pressed().fold(
                    MouseFlags(0),
                    |mut flags, button| {
                        flags.add(button);
                        flags
                    },
                );

                let old_flags =
                    mouse_button_input
                        .get_pressed()
                        .fold(MouseFlags(0), |mut flags, button| {
                            flags.add(button);
                            flags
                        });

                if old_flags.0 == 0
                // nothing is clicked
                {
                    *interaction = Interaction::Hovered;
                } else if new_flags.0 != 0 {
                    *interaction = Interaction::Clicked(old_flags)
                }
            }

            hovered_entity = Some(entity);

            match focus_policy.cloned().unwrap_or(FocusPolicy::Block) {
                FocusPolicy::Block => {
                    break;
                }
                FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
            }
        }
    }

    // if there is a new hovered entity, but an entity is currently hovered, unhover the old entity
    if let Some(new_hovered_entity) = hovered_entity {
        if let Some(old_hovered_entity) = state.hovered_entity {
            if new_hovered_entity != old_hovered_entity {
                if let Ok(mut interaction) = node_query.get_mut::<Interaction>(old_hovered_entity) {
                    if *interaction == Interaction::Hovered {
                        *interaction = Interaction::None;
                    }
                }
                state.hovered_entity = None;
            }
        }
        state.hovered_entity = hovered_entity;
    }
}
