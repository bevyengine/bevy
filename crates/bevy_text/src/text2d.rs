use bevy_asset::Assets;
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    query::{Changed, QueryState, With},
    system::{Local, Query, QuerySet, Res, ResMut},
};
use bevy_math::{Mat4, Size, Vec3};
use bevy_render::{texture::Image, RenderWorld};
use bevy_sprite::{ExtractedSprite, ExtractedSprites, TextureAtlas};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_window::Windows;

use crate::{
    DefaultTextPipeline, Font, FontAtlasSet, HorizontalAlign, Text, Text2dSize, TextError,
    VerticalAlign,
};

/// The bundle of components needed to draw text in a 2D scene via a 2D `OrthographicCameraBundle`.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
#[derive(Bundle, Clone, Debug)]
pub struct Text2dBundle {
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_2d_size: Text2dSize,
}

impl Default for Text2dBundle {
    fn default() -> Self {
        Self {
            text: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            text_2d_size: Text2dSize {
                size: Size::default(),
            },
        }
    }
}

pub fn extract_text2d_sprite(
    mut render_world: ResMut<RenderWorld>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    text_pipeline: Res<DefaultTextPipeline>,
    windows: Res<Windows>,
    mut text2d_query: Query<(Entity, &Text, &GlobalTransform, &Text2dSize)>,
) {
    let mut extracted_sprites = render_world.get_resource_mut::<ExtractedSprites>().unwrap();
    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor() as f32
    } else {
        1.
    };

    for (entity, text, transform, calculated_size) in text2d_query.iter_mut() {
        let (width, height) = (calculated_size.size.width, calculated_size.size.height);

        if let Some(text_layout) = text_pipeline.get_glyphs(&entity) {
            let text_glyphs = &text_layout.glyphs;
            let alignment_offset = match text.alignment.vertical {
                VerticalAlign::Top => Vec3::new(0.0, -height, 0.0),
                VerticalAlign::Center => Vec3::new(0.0, -height * 0.5, 0.0),
                VerticalAlign::Bottom => Vec3::ZERO,
            } + match text.alignment.horizontal {
                HorizontalAlign::Left => Vec3::ZERO,
                HorizontalAlign::Center => Vec3::new(-width * 0.5, 0.0, 0.0),
                HorizontalAlign::Right => Vec3::new(-width, 0.0, 0.0),
            };

            for text_glyph in text_glyphs {
                let color = text.sections[text_glyph.section_index]
                    .style
                    .color
                    .as_rgba_linear();
                let atlas = texture_atlases
                    .get(text_glyph.atlas_info.texture_atlas.clone_weak())
                    .unwrap();
                let handle = atlas.texture.clone_weak();
                let index = text_glyph.atlas_info.glyph_index as usize;
                let rect = atlas.textures[index];
                let atlas_size = Some(atlas.size);

                let transform =
                    Mat4::from_rotation_translation(transform.rotation, transform.translation)
                        * Mat4::from_scale(transform.scale / scale_factor)
                        * Mat4::from_translation(
                            alignment_offset * scale_factor + text_glyph.position.extend(0.),
                        );

                extracted_sprites.sprites.push(ExtractedSprite {
                    transform,
                    color,
                    rect,
                    handle,
                    atlas_size,
                    flip_x: false,
                    flip_y: false,
                });
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct QueuedText2d {
    entities: Vec<Entity>,
}

/// Updates the TextGlyphs with the new computed glyphs from the layout
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn text2d_system(
    mut queued_text: Local<QueuedText2d>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: QuerySet<(
        QueryState<Entity, (With<Text2dSize>, Changed<Text>)>,
        QueryState<(&Text, &mut Text2dSize), With<Text2dSize>>,
    )>,
) {
    // Adds all entities where the text or the style has changed to the local queue
    for entity in text_queries.q0().iter_mut() {
        queued_text.entities.push(entity);
    }

    if queued_text.entities.is_empty() {
        return;
    }

    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor()
    } else {
        1.
    };

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.q1();
    for entity in queued_text.entities.drain(..) {
        if let Ok((text, mut calculated_size)) = query.get_mut(entity) {
            match text_pipeline.queue_text(
                entity,
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                Size::new(f32::MAX, f32::MAX),
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
                        width: scale_value(text_layout_info.size.width, 1. / scale_factor),
                        height: scale_value(text_layout_info.size.height, 1. / scale_factor),
                    };
                }
            }
        }
    }

    queued_text.entities = new_queue;
}

pub fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
