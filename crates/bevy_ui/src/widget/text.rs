use crate::{CalculatedSize, Node, Size, Style, UiScale, Val};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With},
    system::{Local, ParamSet, Query, Res, ResMut},
};

use bevy_hierarchy::Parent;
use bevy_math::Vec2;
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

/// Defines how `min_size`, `size`, and `max_size` affects the bounds of a text
/// block.
pub fn text_constraint(
    min_size: Val,
    size: Val,
    max_size: Val,
    scale_factor: f64,
    container_size: f32,
) -> f32 {
    // Needs support for percentages
    match (min_size, size, max_size) {
        (_, _, Val::Px(max)) => scale_value(max, scale_factor),
        (_, _, Val::Percent(max)) => scale_value((max / 100.) * container_size, scale_factor),
        (Val::Px(min), _, _) => scale_value(min, scale_factor),
        (Val::Percent(min), _, _) => scale_value((min / 100.) * container_size, scale_factor),
        (Val::Undefined, Val::Px(size), Val::Undefined) | (Val::Auto, Val::Px(size), Val::Auto) => {
            scale_value(size, scale_factor)
        }
        (Val::Undefined, Val::Percent(size), Val::Undefined)
        | (Val::Auto, Val::Percent(size), Val::Auto) => {
            scale_value((size / 100.) * container_size, scale_factor)
        }
        _ => f32::MAX,
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut queued_text: Local<QueuedText>,
    mut should_recalculate_container: Local<bool>,
    mut last_window_details: Local<(f64, f32, f32)>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Or<(Changed<Text>, Changed<Style>)>>,
        Query<Entity, (With<Text>, With<Style>)>,
        Query<(&Text, &Style, &mut CalculatedSize, Option<&Parent>)>,
    )>,
    node_query: Query<&Node>,
) {
    // TODO: This should support window-independent scale settings.
    // See https://github.com/bevyengine/bevy/issues/5621
    let (scale_factor, window_width_constraint, window_height_constraint) =
        if let Some(window) = windows.get_primary() {
            (
                window.scale_factor() * ui_scale.scale,
                window.width(),
                window.height(),
            )
        } else {
            (1., f32::MAX, f32::MAX)
        };

    let inv_scale_factor = 1. / scale_factor;

    #[allow(clippy::float_cmp)]
    if last_window_details.0 == scale_factor
        && last_window_details.1 == window_width_constraint
        && last_window_details.2 == window_height_constraint
        && !*should_recalculate_container
    {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            queued_text.entities.push(entity);
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.entities.push(entity);
        }
        last_window_details.0 = scale_factor;
        last_window_details.1 = window_width_constraint;
        last_window_details.2 = window_height_constraint;
    }

    if queued_text.entities.is_empty() {
        return;
    }

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text.entities.drain(..) {
        if let Ok((text, style, mut calculated_size, parent)) = query.get_mut(entity) {
            let mut width_contraint = window_width_constraint;
            let mut height_contraint = window_height_constraint;
            if let Some(parent) = parent {
                if let Ok(node) = node_query.get(parent.get()) {
                    if node.size == Vec2::default() {
                        // Because the nodes size gets calculated after we run this system,
                        // we need to rerun this system to calculate the correct bounds
                        *should_recalculate_container = true;
                    } else {
                        width_contraint = node.size.x;
                        height_contraint = node.size.y;
                    }
                }
            }

            let node_size = Vec2::new(
                text_constraint(
                    style.min_size.width,
                    style.size.width,
                    style.max_size.width,
                    scale_factor,
                    width_contraint,
                ),
                text_constraint(
                    style.min_size.height,
                    style.size.height,
                    style.max_size.height,
                    scale_factor,
                    height_contraint,
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
                        width: Val::Px(scale_value(text_layout_info.size.x, inv_scale_factor)),
                        height: Val::Px(scale_value(text_layout_info.size.y, inv_scale_factor)),
                    };
                }
            }
        }
    }

    queued_text.entities = new_queue;
}

#[cfg(test)]
mod tests {
    use super::text_constraint;
    use crate::Val;

    #[test]
    fn should_constrain_based_on_pixel_values() {
        assert_eq!(
            text_constraint(Val::Px(100.), Val::Undefined, Val::Undefined, 1., 1.),
            100.
        );
        assert_eq!(
            text_constraint(Val::Undefined, Val::Undefined, Val::Px(100.), 1., 1.),
            100.
        );
        assert_eq!(
            text_constraint(Val::Undefined, Val::Px(100.), Val::Undefined, 1., 1.),
            100.
        );
    }

    #[test]
    fn should_constrain_based_on_percent_values() {
        assert_eq!(
            text_constraint(Val::Percent(33.), Val::Undefined, Val::Undefined, 1., 1000.),
            330.
        );
        assert_eq!(
            text_constraint(Val::Undefined, Val::Undefined, Val::Percent(33.), 1., 1000.),
            330.
        );
        assert_eq!(
            text_constraint(Val::Undefined, Val::Percent(33.), Val::Undefined, 1., 1000.),
            330.
        );
    }

    #[test]
    fn should_ignore_min_if_max_is_given() {
        assert_eq!(
            text_constraint(
                Val::Percent(33.),
                Val::Undefined,
                Val::Percent(50.),
                1.,
                1000.
            ),
            500.,
            "min in percent and max in percent"
        );
        assert_eq!(
            text_constraint(Val::Px(33.), Val::Undefined, Val::Px(50.), 1., 1000.),
            50.,
            "min in px and max in px"
        );
        assert_eq!(
            text_constraint(Val::Px(33.), Val::Undefined, Val::Percent(50.), 1., 1000.),
            500.,
            "min in px and max in percent"
        );
        assert_eq!(
            text_constraint(Val::Percent(33.), Val::Undefined, Val::Px(50.), 1., 1000.),
            50.,
            "min in percent and max in px"
        );
    }
}
