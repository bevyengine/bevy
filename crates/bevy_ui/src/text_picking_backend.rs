use bevy_ecs::{
    event::Event,
    observer::Trigger,
    system::{Commands, Query},
};
use bevy_picking::events::Pointer;
use bevy_reflect::Reflect;
use bevy_text::{cosmic_text::Cursor, ComputedTextBlock};

use crate::{ComputedNode, RelativeCursorPosition};

#[derive(Event, Debug, Clone)]
pub struct TextPointer<E: Clone + Reflect + std::fmt::Debug> {
    pub cursor: Cursor,
    pub event: Pointer<E>,
}

/// Takes UI pointer hits and re-emits them as `TextPointer` triggers.
pub(crate) fn get_and_emit_text_hits<E: Clone + Reflect + std::fmt::Debug>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(&ComputedNode, &RelativeCursorPosition, &ComputedTextBlock)>,
    mut commands: Commands,
) {
    if q.get(trigger.target()).is_err() {
        return;
    }
    // Get click position relative to node
    let (c_node, pos, c_text) = q
        .get(trigger.target())
        .expect("missing required component(s)");

    let Some(hit_pos) = pos.normalized else {
        return;
    };

    let physical_pos = hit_pos * c_node.size;

    let Some(cursor) = c_text.buffer().hit(physical_pos.x, physical_pos.y) else {
        return;
    };

    // TODO: trigger targeted span entities, might need to have PositionedGlyph at this point?
    commands.trigger_targets(
        TextPointer::<E> {
            cursor,
            event: trigger.event().clone(),
        },
        trigger.target(),
    );
}
