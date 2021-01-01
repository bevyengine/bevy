use bevy_asset::Assets;
use bevy_ecs::{Bundle, Changed, Entity, Local, Query, QuerySet, Res, ResMut, With};
use bevy_math::{Size, Vec3};
use bevy_render::{
    draw::{DrawContext, Drawable},
    mesh::Mesh,
    prelude::{Draw, Msaa, Texture, Visible},
    render_graph::base::MainPass,
    renderer::RenderResourceBindings,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_window::Windows;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

use crate::{
    CalculatedSize, DefaultTextPipeline, DrawableText, Font, FontAtlasSet, Text, TextError,
};

/// The bundle of components needed to draw text in a 2D scene via the Camera2dBundle.
#[derive(Bundle, Clone, Debug)]
pub struct Text2dBundle {
    pub draw: Draw,
    pub visible: Visible,
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub main_pass: MainPass,
    pub calculated_size: CalculatedSize,
}

impl Default for Text2dBundle {
    fn default() -> Self {
        Self {
            draw: Draw {
                ..Default::default()
            },
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            text: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            main_pass: MainPass {},
            calculated_size: CalculatedSize {
                size: Size::default(),
            },
        }
    }
}

/// System for drawing text in a 2D scene via the Camera2dBundle.  Included in the default
/// `TextPlugin`. Position is determined by the `Transform`'s translation, though scale and rotation
/// are ignored.
pub fn draw_text2d_system(
    mut context: DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    windows: Res<Windows>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    text_pipeline: Res<DefaultTextPipeline>,
    mut query: Query<
        (
            Entity,
            &mut Draw,
            &Visible,
            &Text,
            &GlobalTransform,
            &CalculatedSize,
        ),
        With<MainPass>,
    >,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor() as f32
    } else {
        1.
    };

    for (entity, mut draw, visible, text, global_transform, calculated_size) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        let (width, height) = (calculated_size.size.width, calculated_size.size.height);

        if let Some(text_glyphs) = text_pipeline.get_glyphs(&entity) {
            let position = global_transform.translation
                + match text.style.alignment.vertical {
                    VerticalAlign::Top => Vec3::zero(),
                    VerticalAlign::Center => Vec3::new(0.0, -height * 0.5, 0.0),
                    VerticalAlign::Bottom => Vec3::new(0.0, -height, 0.0),
                }
                + match text.style.alignment.horizontal {
                    HorizontalAlign::Left => Vec3::new(-width, 0.0, 0.0),
                    HorizontalAlign::Center => Vec3::new(-width * 0.5, 0.0, 0.0),
                    HorizontalAlign::Right => Vec3::zero(),
                };

            let mut drawable_text = DrawableText {
                render_resource_bindings: &mut render_resource_bindings,
                position,
                msaa: &msaa,
                text_glyphs: &text_glyphs.glyphs,
                font_quad_vertex_descriptor: &vertex_buffer_descriptor,
                scale_factor,
                style: &text.style,
            };

            drawable_text.draw(&mut draw, &mut context).unwrap();
        }
    }
}

#[derive(Debug, Default)]
pub struct QueuedText2d {
    entities: Vec<Entity>,
}

/// Updates the TextGlyphs with the new computed glyphs from the layout
#[allow(clippy::too_many_arguments)]
pub fn text2d_system(
    mut queued_text: Local<QueuedText2d>,
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: QuerySet<(
        Query<Entity, (With<MainPass>, Changed<Text>)>,
        Query<(&Text, &mut CalculatedSize), With<MainPass>>,
    )>,
) {
    // Adds all entities where the text or the style has changed to the local queue
    for entity in text_queries.q0_mut().iter_mut() {
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
    let query = text_queries.q1_mut();
    for entity in queued_text.entities.drain(..) {
        if let Ok((text, mut calculated_size)) = query.get_mut(entity) {
            match text_pipeline.queue_text(
                entity,
                text.font.clone(),
                &fonts,
                &text.value,
                scale_value(text.style.font_size, scale_factor),
                text.style.alignment,
                Size::new(f32::MAX, f32::MAX),
                &mut *font_atlas_set_storage,
                &mut *texture_atlases,
                &mut *textures,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the queue for further processing
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

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
