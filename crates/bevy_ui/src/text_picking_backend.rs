use bevy_app::App;
use bevy_ecs::{
    event::Event,
    observer::Trigger,
    system::{Commands, Query},
};
use bevy_math::Rect;
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::Reflect;
use bevy_text::{PositionedGlyph, TextLayoutInfo};

use crate::{ComputedNode, RelativeCursorPosition};

#[derive(Event, Debug, Clone, Reflect)]
pub struct TextPointer<E: Clone + Reflect + std::fmt::Debug> {
    positioned_glyph: PositionedGlyph,
    event: Pointer<E>,
}

/// Takes UI pointer hits and re-emits them as `TextPointer` triggers.
pub(crate) fn get_and_emit_text_hits<E: Clone + Reflect + std::fmt::Debug>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(&ComputedNode, &TextLayoutInfo, &RelativeCursorPosition)>,
    mut commands: Commands,
) {
    if q.get(trigger.target()).is_err() {
        return;
    }
    // Get click position relative to node
    let (c_node, text, pos) = q
        .get(trigger.target())
        .expect("missing required component(s)");

    let Some(hit_pos) = pos.normalized else {
        return;
    };

    let Some(positioned_glyph) = text.glyphs.iter().find_map(|g| {
        // TODO: fullheight rects, use g.position and c_node tings
        // TODO: spaces, fill from previous rect to next rect... somehow
        let rect = Rect::from_corners(g.position - g.size / 2.0, g.position + g.size / 2.0);

        if rect.contains(hit_pos * c_node.size()) {
            Some(g)
        } else {
            None
        }
    }) else {
        return;
    };

    commands.trigger_targets(
        TextPointer::<E> {
            positioned_glyph: positioned_glyph.clone(),
            event: trigger.event().clone(),
        },
        trigger.target(),
    );
}
