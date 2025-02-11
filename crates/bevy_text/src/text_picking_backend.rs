use bevy_app::App;
use bevy_ecs::{
    observer::Trigger,
    system::{Commands, Query},
};
use bevy_picking::events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Move, Out,
    Over, Pointer, Pressed, Released,
};
use bevy_reflect::Reflect;
use bevy_sprite::Anchor;

use crate::{ComputedTextBlock, TextBounds, TextLayoutInfo};

pub(crate) fn plugin(app: &mut App) {
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

pub(crate) fn get_and_emit_text_hits<E: Clone + Reflect + std::fmt::Debug>(
    trigger: Trigger<Pointer<E>>,
    q: Query<(&ComputedTextBlock, &TextLayoutInfo, &Anchor, &TextBounds)>,
    mut commands: Commands,
) {
    let Ok((c_text, text_layout, anchor, bounds)) = q.get(trigger.target) else {
        return;
    };
}
