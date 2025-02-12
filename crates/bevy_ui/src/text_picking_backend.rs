use bevy_app::App;
use bevy_ecs::{
    observer::Trigger,
    system::{Commands, Query},
};
use bevy_picking::events::{
    Cancel, Click, DragDrop, DragEnter, DragLeave, DragOver, DragStart, Move, Out, Over, Pointer,
    Pressed, Released,
};
use bevy_text::{
    text_pointer::{HasHit, TextPointer},
    ComputedTextBlock, TextLayoutInfo,
};

use crate::ComputedNode;

pub(crate) fn plugin(app: &mut App) {
    app.add_observer(get_and_emit_text_hits::<Cancel>)
        .add_observer(get_and_emit_text_hits::<Click>)
        .add_observer(get_and_emit_text_hits::<Pressed>)
        .add_observer(get_and_emit_text_hits::<DragDrop>)
        .add_observer(get_and_emit_text_hits::<DragEnter>)
        .add_observer(get_and_emit_text_hits::<DragLeave>)
        .add_observer(get_and_emit_text_hits::<DragOver>)
        .add_observer(get_and_emit_text_hits::<DragStart>)
        .add_observer(get_and_emit_text_hits::<Move>)
        .add_observer(get_and_emit_text_hits::<Out>)
        .add_observer(get_and_emit_text_hits::<Over>)
        .add_observer(get_and_emit_text_hits::<Released>);

    // TODO: investigate whether hit data can be added here
    // + investigate if drag events are ever useful?
    // .add_observer(get_and_emit_text_hits::<DragEnd>)
    // .add_observer(get_and_emit_text_hits::<Drag>)
}

/// Takes UI pointer hits and re-emits them as `TextPointer` triggers.
pub(crate) fn get_and_emit_text_hits<E: HasHit>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(&ComputedNode, &ComputedTextBlock, &TextLayoutInfo)>,
    mut commands: Commands,
) {
    // Get click position relative to node
    let Ok((c_node, c_text, text_layout)) = q.get(trigger.target()) else {
        return;
    };

    let Some(hit_pos) = trigger.event.hit().position else {
        return;
    };

    let physical_pos = hit_pos.truncate() * c_node.size;

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

    let text_pointer = TextPointer::<E> {
        cursor,
        // TODO: can this be a borrow?
        glyph: positioned_glyph.clone(),
        event: trigger.event().clone(),
    };

    commands.trigger_targets(text_pointer.clone(), target_span.entity);
}
