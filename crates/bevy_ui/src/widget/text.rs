use crate::{CalculatedSize, Size, Style, UiScale, Val};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With},
    system::{Commands, Local, ParamSet, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo, TextPipeline,
    TextSettings, YAxisOrientation,
};
use bevy_window::Windows;

#[derive(Debug, Default)]
pub struct QueuedText {
    entities: Vec<Entity>,
}

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

/// Defines how `min_size`, `size`, and `max_size` affects the bounds of a text
/// block.
pub fn text_constraint(min_size: Val, size: Val, max_size: Val, scale_factor: f64) -> f32 {
    // Needs support for percentages
    match (min_size, size, max_size) {
        (_, _, Val::Px(max)) => scale_value(max, scale_factor),
        (Val::Px(min), _, _) => scale_value(min, scale_factor),
        (Val::Undefined, Val::Px(size), Val::Undefined) | (Val::Auto, Val::Px(size), Val::Auto) => {
            scale_value(size, scale_factor)
        }
        _ => f32::MAX,
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut commands: Commands,
    mut queued_text: Local<QueuedText>,
    mut last_scale_factor: Local<f64>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Or<(Changed<Text>, Changed<Style>)>>,
        Query<Entity, (With<Text>, With<Style>)>,
        Query<(
            &Text,
            &Style,
            &mut CalculatedSize,
            Option<&mut TextLayoutInfo>,
        )>,
    )>,
) {
    // TODO: This should support window-independent scale settings.
    // See https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor() * ui_scale.scale
    } else {
        ui_scale.scale
    };

    let inv_scale_factor = 1. / scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            queued_text.entities.push(entity);
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.entities.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text.entities.is_empty() {
        return;
    }

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text.entities.drain(..) {
        if let Ok((text, style, mut calculated_size, text_layout_info)) = query.get_mut(entity) {
            let node_size = Vec2::new(
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
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                node_size,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                &mut font_atlas_warning,
                YAxisOrientation::TopToBottom,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_))
                | Err(e @ TextError::ExceedMaxTextAtlases(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    calculated_size.size = Size {
                        width: Val::Px(scale_value(info.size.x, inv_scale_factor)),
                        height: Val::Px(scale_value(info.size.y, inv_scale_factor)),
                    };
                    match text_layout_info {
                        Some(mut t) => *t = info,
                        None => {
                            commands.entity(entity).insert(info);
                        }
                    }
                }
            }
        }
    }

    queued_text.entities = new_queue;
}
