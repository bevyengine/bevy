use bevy_app::App;
use bevy_ecs::{
    event::Event,
    observer::Trigger,
    system::{Commands, Query},
};
use bevy_picking::events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Move, Out,
    Over, Pointer, Pressed, Released,
};
use bevy_reflect::Reflect;
use bevy_text::{cosmic_text::Cursor, ComputedTextBlock, TextLayoutInfo};

use crate::{ComputedNode, RelativeCursorPosition};

pub(crate) fn plugin(app: &mut App) {
    app.add_event::<TextPointer<Cancel>>()
        .add_event::<TextPointer<Click>>()
        .add_event::<TextPointer<Pressed>>()
        .add_event::<TextPointer<DragDrop>>()
        .add_event::<TextPointer<DragEnd>>()
        .add_event::<TextPointer<DragEnter>>()
        .add_event::<TextPointer<Drag>>()
        .add_event::<TextPointer<DragLeave>>()
        .add_event::<TextPointer<DragOver>>()
        .add_event::<TextPointer<DragStart>>()
        .add_event::<TextPointer<Move>>()
        .add_event::<TextPointer<Out>>()
        .add_event::<TextPointer<Over>>()
        .add_event::<TextPointer<Released>>();

    app.add_observer(get_and_emit_text_hits::<Cancel>)
        .add_observer(get_and_emit_text_hits::<Click>)
        .add_observer(get_and_emit_text_hits::<Pressed>)
        .add_observer(get_and_emit_text_hits::<DragDrop>)
        .add_observer(get_and_emit_text_hits::<DragEnd>)
        .add_observer(get_and_emit_text_hits::<DragEnter>)
        .add_observer(get_and_emit_text_hits::<Drag>)
        .add_observer(get_and_emit_text_hits::<DragLeave>)
        .add_observer(get_and_emit_text_hits::<DragOver>)
        .add_observer(get_and_emit_text_hits::<DragStart>)
        .add_observer(get_and_emit_text_hits::<Move>)
        .add_observer(get_and_emit_text_hits::<Out>)
        .add_observer(get_and_emit_text_hits::<Over>)
        .add_observer(get_and_emit_text_hits::<Released>);
}

#[derive(Event, Debug, Clone)]
pub struct TextPointer<E: Clone + Reflect + std::fmt::Debug> {
    pub cursor: Cursor,
    pub event: Pointer<E>,
}

/// Takes UI pointer hits and re-emits them as `TextPointer` triggers.
pub(crate) fn get_and_emit_text_hits<E: Clone + Reflect + std::fmt::Debug>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(
        &ComputedNode,
        &RelativeCursorPosition,
        &ComputedTextBlock,
        &TextLayoutInfo,
    )>,
    mut commands: Commands,
) {
    if q.get(trigger.target()).is_err() {
        return;
    }
    // Get click position relative to node
    let (c_node, pos, c_text, text_layout) = q
        .get(trigger.target())
        .expect("missing required component(s)");

    let Some(hit_pos) = pos.normalized else {
        return;
    };

    let physical_pos = hit_pos * c_node.size;

    let Some(cursor) = c_text.buffer().hit(physical_pos.x, physical_pos.y) else {
        return;
    };

    // PERF: doing this as well as using cosmic's `hit` is the worst of both worlds. This approach
    // allows for span-specific events, whereas cosmic's hit detection is faster by discarding
    // per-line, and also gives cursor affinity (direction on glyph)
    let Some(positioned_glyph) = text_layout
        .glyphs
        .iter()
        .find(|g| g.byte_index == cursor.index && g.line_index == cursor.line)
    else {
        return;
    };

    // Get span entity
    let target_span = c_text.entities()[positioned_glyph.span_index];

    // TODO: consider sending the `PositionedGlyph` along with the event
    let text_pointer = TextPointer::<E> {
        cursor,
        event: trigger.event().clone(),
    };

    commands.trigger_targets(text_pointer.clone(), target_span.entity);

    // If span == 0, this event was sent already, so skip. This second dispatch means that an
    // observer only added to the root text entity still triggers when child spans are interacted
    // with.
    // TODO: i think event propagation could be useful here?
    if positioned_glyph.span_index != 0 {
        commands.trigger_targets(text_pointer.clone(), trigger.target());
    }
}
