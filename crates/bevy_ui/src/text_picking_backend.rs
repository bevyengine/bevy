use bevy_app::App;
use bevy_ecs::{
    entity::{Entity, EntityBorrow},
    event::Event,
    hierarchy::ChildOf,
    observer::Trigger,
    query::QueryData,
    system::{Commands, Query},
    traversal::Traversal,
};
use bevy_picking::events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Move, Out,
    Over, Pointer, Pressed, Released,
};
use bevy_reflect::Reflect;
use bevy_render::camera::NormalizedRenderTarget;
use bevy_text::{cosmic_text::Cursor, ComputedTextBlock, PositionedGlyph, TextLayoutInfo};
use bevy_window::Window;

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

#[derive(Debug, Clone)]
pub struct TextPointer<E: Clone + Reflect + std::fmt::Debug> {
    pub cursor: Cursor,
    pub glyph: PositionedGlyph,
    pub event: Pointer<E>,
}

impl<E> Event for TextPointer<E>
where
    E: Clone + Reflect + std::fmt::Debug,
{
    const AUTO_PROPAGATE: bool = true;
    type Traversal = TextPointerTraversal;
}

/// A traversal query (eg it implements [`Traversal`]) intended for use with [`TextPointer`] events.
///
/// This will always traverse to the parent, if the entity being visited has one. Otherwise, it
/// propagates to the pointer's window and stops there.
#[derive(QueryData)]
pub struct TextPointerTraversal {
    parent: Option<&'static ChildOf>,
    window: Option<&'static Window>,
}

impl<E> Traversal<TextPointer<E>> for TextPointerTraversal
where
    E: std::fmt::Debug + Clone + Reflect,
{
    fn traverse(item: Self::Item<'_>, pointer: &TextPointer<E>) -> Option<Entity> {
        let TextPointerTraversalItem { parent, window } = item;

        // Send event to parent, if it has one.
        if let Some(parent) = parent {
            return Some(parent.get());
        };

        // Otherwise, send it to the window entity (unless this is a window entity).
        if window.is_none() {
            if let NormalizedRenderTarget::Window(window_ref) =
                pointer.event.pointer_location.target
            {
                return Some(window_ref.entity());
            }
        }

        None
    }
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
    // Get click position relative to node
    let Ok((c_node, pos, c_text, text_layout)) = q.get(trigger.target()) else {
        return;
    };

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

    let text_pointer = TextPointer::<E> {
        cursor,
        // TODO: can this be a borrow?
        glyph: positioned_glyph.clone(),
        event: trigger.event().clone(),
    };

    commands.trigger_targets(text_pointer.clone(), target_span.entity);
}
