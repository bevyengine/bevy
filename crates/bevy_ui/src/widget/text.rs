use crate::{CalculatedSize, Style, Val};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    prelude::QueryState,
    query::{Changed, Or, With},
    system::{Local, QuerySet, Res, ResMut},
};
use bevy_math::Size;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{DefaultTextPipeline, Font, FontAtlasSet, Text, TextError};
use bevy_window::Windows;

#[derive(Debug, Default)]
pub struct QueuedText {
    entities: Vec<Entity>,
}

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

/// Defines how min_size, size, and max_size affects the bounds of a text
/// block.
pub fn text_constraint(min_size: Val, size: Val, max_size: Val, scale_factor: f64) -> f32 {
    // Needs support for percentages
    match (min_size, size, max_size) {
        (_, _, Val::Px(max)) => scale_value(max, scale_factor),
        (Val::Px(min), _, _) => scale_value(min, scale_factor),
        (Val::Undefined, Val::Px(size), Val::Undefined) => scale_value(size, scale_factor),
        (Val::Auto, Val::Px(size), Val::Auto) => scale_value(size, scale_factor),
        _ => f32::MAX,
    }
}

/// Computes the size of a text block and updates the TextGlyphs with the
/// new computed glyphs from the layout
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn text_system(
    mut queued_text: Local<QueuedText>,
    mut last_scale_factor: Local<f64>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: QuerySet<(
        QueryState<Entity, Or<(Changed<Text>, Changed<Style>)>>,
        QueryState<Entity, (With<Text>, With<Style>)>,
        QueryState<(&Text, &Style, &mut CalculatedSize)>,
    )>,
) {
    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor()
    } else {
        1.
    };

    let inv_scale_factor = 1. / scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.q0().iter() {
            queued_text.entities.push(entity);
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.q1().iter() {
            queued_text.entities.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text.entities.is_empty() {
        return;
    }

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.q2();
    for entity in queued_text.entities.drain(..) {
        if let Ok((text, style, mut calculated_size)) = query.get_mut(entity) {
            let node_size = Size::new(
                text_constraint(
                    style.min_size.width,
                    style.size.width,
                    style.max_size.width,
                    scale_factor,
                ),
                text_constraint(
                    style.min_size.height,
                    style.size.height,
                    style.max_size.height,
                    scale_factor,
                ),
            );

            match text_pipeline.queue_text(
                entity,
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                node_size,
                &mut *font_atlas_set_storage,
                &mut *texture_atlases,
                &mut *textures,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {}.", e);
                }
                Ok(()) => {
                    let text_layout_info = text_pipeline.get_glyphs(&entity).expect(
                        "Failed to get glyphs from the pipeline that have just been computed",
                    );
                    calculated_size.size = Size {
                        width: scale_value(text_layout_info.size.width, inv_scale_factor),
                        height: scale_value(text_layout_info.size.height, inv_scale_factor),
                    };
                }
            }
        }
    }

    queued_text.entities = new_queue;
}
