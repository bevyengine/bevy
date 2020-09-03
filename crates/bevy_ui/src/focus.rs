use crate::{events::*, Node};
use bevy_app::{EventReader, Events};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButton, Input};
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_window::CursorMoved;

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
    mousedown_reader: EventReader<MouseDown>,
    mouseup_reader: EventReader<MouseUp>,
    mouseenter_reader: EventReader<MouseEnter>,
    mouseleave_reader: EventReader<MouseLeave>,
}

pub fn ui_focus_system(
    mut state: Local<State>,
    ui_events: (
        Res<Events<MouseDown>>,
        Res<Events<MouseUp>>,
        Res<Events<MouseEnter>>,
        Res<Events<MouseLeave>>,
    ),
    mut node_query: Query<(
        Entity,
        &Node,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
    )>,
) {
    let (mousedown_events, mouseup_events, mouseenter_events, mouseleave_events) = ui_events;

    for (entity, _node, interaction, _) in &mut node_query.iter() {
        if let Some(mut interaction) = interaction {
            for event in state.mousedown_reader.iter(&mousedown_events) {
                if event.entity == entity {
                    if *interaction != Interaction::Pressed {
                        *interaction = Interaction::Pressed;
                    }
                }
            }

            for event in state.mouseup_reader.iter(&mouseup_events) {
                if event.entity == entity {
                    if *interaction == Interaction::Pressed {
                        *interaction = Interaction::Hovered;
                    }
                }
            }

            for event in state.mouseleave_reader.iter(&mouseleave_events) {
                if event.entity == entity {
                    *interaction = Interaction::None
                }
            }

            for event in state.mouseenter_reader.iter(&mouseenter_events) {
                if event.entity == entity {
                    if *interaction != Interaction::Hovered {
                        *interaction = Interaction::Hovered;
                    }
                }
            }
        }
    }
}
