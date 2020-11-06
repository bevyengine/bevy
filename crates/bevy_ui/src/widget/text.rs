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
use bevy_text::{DrawableText, Font, FontAtlasSet, TextPipeline, TextStyle, TextVertices};
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

/// Computes the size of a text block and updates the TextVertices with the
/// new computed vertices
pub fn text_system(
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Or<(Changed<Text>, Changed<Style>)>,
        &mut TextVertices,
        &mut CalculatedSize,
    )>,
) {
    for ((text, style), mut vertices, mut calculated_size) in text_query.iter_mut() {
        let node_size = Size::new(
            text_constraint(style.min_size.width, style.size.width, style.max_size.width),
            text_constraint(
                style.min_size.height,
                style.size.height,
                style.max_size.height,
            ),
        );

        if let Err(e) = text_pipeline.queue_text(
            text.font.clone(),
            &fonts,
            &text.value,
            text.style.font_size,
            text.style.alignment,
            node_size,
        ) {
            println!("Error when adding text to the queue: {:?}", e);
        } else if let Ok(new_size) = text_pipeline.measure(
            text.font.clone(),
            &fonts,
            &text.value,
            text.style.font_size,
            text.style.alignment,
            node_size,
        ) {
            calculated_size.size = new_size;
        }

        match text_pipeline.process_queued(
            &fonts,
            &mut font_atlas_sets,
            &mut texture_atlases,
            &mut textures,
        ) {
            Err(e) => println!("Error when drawing text: {:?}", e),
            Ok(new_vertices) => vertices.set(new_vertices),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_system(
    mut context: DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
    mut query: Query<(&mut Draw, &Text, &TextVertices, &Node, &GlobalTransform)>,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    for (mut draw, text, text_vertices, node, global_transform) in query.iter_mut() {
        let position = global_transform.translation - (node.size / 2.0).extend(0.0);

        let mut drawable_text = DrawableText {
            render_resource_bindings: &mut render_resource_bindings,
            asset_render_resource_bindings: &mut asset_render_resource_bindings,
            position,
            msaa: &msaa,
            text_vertices,
            font_quad_vertex_descriptor: &vertex_buffer_descriptor,
            style: &text.style,
        };

        drawable_text.draw(&mut draw, &mut context).unwrap();
    }
}
