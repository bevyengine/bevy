use crate::{CalculatedSize, Node, Style, Val};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Changed, Entity, Or, Query, Res, ResMut};
use bevy_math::Size;
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    mesh::Mesh,
    prelude::Msaa,
    renderer::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_text::{DefaultTextPipeline, DrawableText, Font, FontAtlasSet, TextError, TextStyle};
use bevy_transform::prelude::GlobalTransform;

#[derive(Debug, Default)]
pub struct QueuedText {
    entities: Vec<Entity>,
}

#[derive(Debug, Default, Clone)]
pub struct Text {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

/// Defines how min_size, size, and max_size affects the bounds of a text
/// block.
pub fn text_constraint(min_size: Val, size: Val, max_size: Val) -> f32 {
    // Needs support for percentages
    match (min_size, size, max_size) {
        (_, _, Val::Px(max)) => max,
        (Val::Px(min), _, _) => min,
        (Val::Undefined, Val::Px(size), Val::Undefined) => size,
        (Val::Auto, Val::Px(size), Val::Auto) => size,
        _ => f32::MAX,
    }
}

/// Computes the size of a text block and updates the TextGlyphs with the
/// new computed glyphs from the layout
pub fn text_system(
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    mut text_query: Query<(
        Entity,
        Or<(Changed<Text>, Changed<Style>)>,
        &mut CalculatedSize,
    )>,
) {
    for (entity, (text, style), mut calculated_size) in text_query.iter_mut() {
        let node_size = Size::new(
            text_constraint(style.min_size.width, style.size.width, style.max_size.width),
            text_constraint(
                style.min_size.height,
                style.size.height,
                style.max_size.height,
            ),
        );

        match add_text_to_pipeline(
            entity,
            &*text,
            node_size,
            &mut *textures,
            &*fonts,
            &mut *texture_atlases,
            &mut *font_atlas_set_storage,
            &mut *text_pipeline,
        ) {
            TextPipelineResult::Ok => {
                let text_layout_info = text_pipeline
                    .get_glyphs(&entity)
                    .expect("Failed to get glyphs from the pipeline that have just been computed");
                calculated_size.size = text_layout_info.size;
            }
            TextPipelineResult::Reschedule => {} // TODO: Reschedule the text computing when font is ready
        }
    }
}

enum TextPipelineResult {
    Ok,
    Reschedule,
}

/// Computes the text layout and stores it in the TextPipeline resource.
fn add_text_to_pipeline(
    entity: Entity,
    text: &Text,
    node_size: Size,
    textures: &mut Assets<Texture>,
    fonts: &Assets<Font>,
    texture_atlases: &mut Assets<TextureAtlas>,
    font_atlas_set_storage: &mut Assets<FontAtlasSet>,
    text_pipeline: &mut DefaultTextPipeline,
) -> TextPipelineResult {
    match text_pipeline.queue_text(
        entity,
        text.font.clone(),
        &fonts,
        &text.value,
        text.style.font_size,
        text.style.alignment,
        node_size,
        font_atlas_set_storage,
        texture_atlases,
        textures,
    ) {
        Err(TextError::NoSuchFont) => TextPipelineResult::Reschedule,
        Err(e @ TextError::FailedToAddGlyph(_)) => {
            panic!("Fatal error when processing text: {}", e);
        }
        Ok(()) => TextPipelineResult::Ok,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_system(
    mut context: DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
    text_pipeline: Res<DefaultTextPipeline>,
    mut query: Query<(Entity, &mut Draw, &Text, &Node, &GlobalTransform)>,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    for (entity, mut draw, text, node, global_transform) in query.iter_mut() {
        if let Some(text_glyphs) = text_pipeline.get_glyphs(&entity) {
            let position = global_transform.translation - (node.size / 2.0).extend(0.0);

            let mut drawable_text = DrawableText {
                render_resource_bindings: &mut render_resource_bindings,
                asset_render_resource_bindings: &mut asset_render_resource_bindings,
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
