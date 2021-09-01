use bevy_asset::Assets;
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    query::{Changed, QueryState, With, Without},
    system::{Local, Query, QuerySet, Res, ResMut},
};
use bevy_math::{Size, Vec3};
use bevy_render::{
    draw::{DrawContext, Drawable, OutsideFrustum},
    mesh::Mesh,
    prelude::{Draw, Msaa, Texture, Visible},
    render_graph::base::MainPass,
    renderer::RenderResourceBindings,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_window::Windows;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

use crate::{DefaultTextPipeline, DrawableText, Font, FontAtlasSet, Text, Text2dSize, TextError};

/// The bundle of components needed to draw text in a 2D scene via a 2D `OrthographicCameraBundle`.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
#[derive(Bundle, Clone, Debug)]
pub struct Text2dBundle {
    pub draw: Draw,
    pub visible: Visible,
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub main_pass: MainPass,
    pub text_2d_size: Text2dSize,
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
            text_2d_size: Text2dSize {
                size: Size::default(),
            },
        }
    }
}

/// System for drawing text in a 2D scene via a 2D `OrthographicCameraBundle`. Included in the
/// default `TextPlugin`. Position is determined by the `Transform`'s translation, though scale and
/// rotation are ignored.
#[allow(clippy::type_complexity)]
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
            &Text2dSize,
        ),
        (With<MainPass>, Without<OutsideFrustum>),
    >,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let font_quad_vertex_layout = font_quad.get_vertex_buffer_layout();

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
            let alignment_offset = match text.alignment.vertical {
                VerticalAlign::Top => Vec3::new(0.0, -height, 0.0),
                VerticalAlign::Center => Vec3::new(0.0, -height * 0.5, 0.0),
                VerticalAlign::Bottom => Vec3::ZERO,
            } + match text.alignment.horizontal {
                HorizontalAlign::Left => Vec3::ZERO,
                HorizontalAlign::Center => Vec3::new(-width * 0.5, 0.0, 0.0),
                HorizontalAlign::Right => Vec3::new(-width, 0.0, 0.0),
            };

            let mut drawable_text = DrawableText {
                render_resource_bindings: &mut render_resource_bindings,
                global_transform: *global_transform,
                scale_factor,
                msaa: &msaa,
                text_glyphs: &text_glyphs.glyphs,
                font_quad_vertex_layout: &font_quad_vertex_layout,
                sections: &text.sections,
                alignment_offset,
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn text2d_system(
    mut queued_text: Local<QueuedText2d>,
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: QuerySet<(
        QueryState<Entity, (With<MainPass>, Changed<Text>)>,
        QueryState<(&Text, &mut Text2dSize), With<MainPass>>,
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
