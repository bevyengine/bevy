use bevy_asset::{Assets, Handle};
use bevy_ecs::{Bundle, Changed, Entity, Local, Query, QuerySet, Res, ResMut, With};
use bevy_math::Size;
use bevy_render::{
    draw::{DrawContext, Drawable},
    mesh::Mesh,
    prelude::{Draw, Msaa, Texture, Visible},
    render_graph::base::MainPass,
    renderer::RenderResourceBindings,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::{DefaultTextPipeline, DrawableText, Font, FontAtlasSet, TextError, TextStyle};

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
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct Text2dBundle {
    pub draw: Draw,
    pub visible: Visible,
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub main_pass: MainPass,
}

// TODO: DRY -- this is copy pasta from bevy_ui/src/widget/text.rs
#[derive(Debug, Default, Clone)]
pub struct Text {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

pub fn draw_text2d_system(
    mut context: DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    text_pipeline: Res<DefaultTextPipeline>,
    mut query: Query<(Entity, &mut Draw, &Visible, &Text, &GlobalTransform), With<MainPass>>,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    for (entity, mut draw, visible, text, global_transform) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        if let Some(text_glyphs) = text_pipeline.get_glyphs(&entity) {
            let position = global_transform.translation; // - (node.size / 2.0).extend(0.0);

            let mut drawable_text = DrawableText {
                render_resource_bindings: &mut render_resource_bindings,
                position,
                msaa: &msaa,
                text_glyphs: &text_glyphs.glyphs,
                font_quad_vertex_descriptor: &vertex_buffer_descriptor,
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
pub fn text2d_system(
    mut queued_text: Local<QueuedText2d>,
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_queries: QuerySet<(Query<Entity, Changed<Text>>, Query<&Text>)>,
) {
    // Adds all entities where the text or the style has changed to the local queue
    for entity in text_queries.q0_mut().iter_mut() {
        queued_text.entities.push(entity);
    }

    if queued_text.entities.is_empty() {
        return;
    }

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let query = text_queries.q1_mut();
    for entity in queued_text.entities.drain(..) {
        if let Ok(text) = query.get_mut(entity) {
            match add_text_to_pipeline(
                entity,
                &*text,
                &mut *textures,
                &*fonts,
                &mut *texture_atlases,
                &mut *font_atlas_set_storage,
                &mut *text_pipeline,
            ) {
                TextPipelineResult::Ok => {
                    // let text_layout_info = text_pipeline.get_glyphs(&entity).expect(
                    //     "Failed to get glyphs from the pipeline that have just been computed",
                    // );
                    //calculated_size.size = text_layout_info.size;
                }
                TextPipelineResult::Reschedule => {
                    // There was an error processing the text layout, let's add this entity to the queue for further processing
                    new_queue.push(entity);
                }
            }
        }
    }

    queued_text.entities = new_queue;
}

// TODO: DRY - this is copy pasta from bevy_ui/src/widget/text.rs
enum TextPipelineResult {
    Ok,
    Reschedule,
}

/// Computes the text layout and stores it in the TextPipeline resource.
#[allow(clippy::too_many_arguments)]
fn add_text_to_pipeline(
    entity: Entity,
    text: &Text,
    textures: &mut Assets<Texture>,
    fonts: &Assets<Font>,
    texture_atlases: &mut Assets<TextureAtlas>,
    font_atlas_set_storage: &mut Assets<FontAtlasSet>,
    text_pipeline: &mut DefaultTextPipeline,
) -> TextPipelineResult {
    // let node_size = Size::new(
    //     text_constraint(style.min_size.width, style.size.width, style.max_size.width),
    //     text_constraint(
    //         style.min_size.height,
    //         style.size.height,
    //         style.max_size.height,
    //     ),
    // );

    // How do we get the actual bounds?  Do we need actual bounds?
    let bounds = Size::new(f32::MAX, f32::MAX);

    match text_pipeline.queue_text(
        entity,
        text.font.clone(),
        &fonts,
        &text.value,
        text.style.font_size,
        text.style.alignment,
        bounds,
        font_atlas_set_storage,
        texture_atlases,
        textures,
    ) {
        Err(TextError::NoSuchFont) => TextPipelineResult::Reschedule,
        Err(e @ TextError::FailedToAddGlyph(_)) => {
            panic!("Fatal error when processing text: {}.", e);
        }
        Ok(()) => TextPipelineResult::Ok,
    }
}
