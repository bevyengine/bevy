//! Text picking backend for `Text2d`.

use bevy_app::App;
use bevy_ecs::{
    observer::Trigger,
    query::With,
    system::{Commands, Query},
};
use bevy_math::Vec2;
use bevy_picking::events::{
    Cancel, Click, DragDrop, DragEnter, DragLeave, DragOver, DragStart, Move, Out, Over, Pointer,
    Pressed, Released,
};
use bevy_render::camera::{Camera, Projection};
use bevy_transform::components::GlobalTransform;

use crate::{
    picking_backend::{get_relative_cursor_pos, rays_from_cursor_camera},
    text_pointer::{HasHit, TextPointer},
    ComputedTextBlock, Text2d, TextBounds, TextLayoutInfo,
};

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
}

// BUG: this double triggers the text events for some reason?
pub(crate) fn get_and_emit_text_hits<E: HasHit>(
    trigger: Trigger<Pointer<E>>,
    q: Query<
        (
            &ComputedTextBlock,
            &TextLayoutInfo,
            &TextBounds,
            &GlobalTransform,
        ),
        With<Text2d>,
    >,
    cam_q: Query<(&Camera, &GlobalTransform, &Projection)>,
    mut commands: Commands,
) {
    let Ok((c_text, text_layout, bounds, transform)) = q.get(trigger.target) else {
        return;
    };

    let Ok((camera, cam_transform, Projection::Orthographic(cam_proj))) =
        cam_q.get(trigger.event.hit().camera)
    else {
        return;
    };

    // TODO: this is duplicate work, pointer already did it
    // Can we just hit_pos-transform here?
    let Some((world_ray, end_ray)) = rays_from_cursor_camera(
        trigger.pointer_location.position,
        camera,
        cam_transform,
        cam_proj,
    ) else {
        return;
    };

    let Some(mut local_pos) = get_relative_cursor_pos(transform, world_ray, end_ray) else {
        return;
    };

    // BUG: incorrect hits reported from this math, scaling issue?
    let size = Vec2::new(
        bounds.width.unwrap_or(text_layout.size.x),
        bounds.height.unwrap_or(text_layout.size.y),
    );

    local_pos.y *= -1.;
    local_pos += size / 2.;

    // TODO: DRY: this is repeated in UI text picking

    let Some(cursor) = c_text.buffer().hit(local_pos.x, local_pos.y) else {
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
        // TODO: getting positioned_glyph can (+ should) be a helper fn (on TextPointer?)
        glyph: positioned_glyph.clone(),
        event: trigger.event().clone(),
    };

    commands.trigger_targets(text_pointer.clone(), target_span.entity);
}
