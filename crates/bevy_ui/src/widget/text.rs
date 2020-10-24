use crate::{CalculatedSize, Node};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Changed, Entity, Local, Query, QuerySet, Res, ResMut, Resource};
use bevy_math::{Size, Vec2};
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    mesh::Mesh,
    prelude::Msaa,
    renderer::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_text::{
    DrawableText, Font, FontAtlasSet, TextDrawer, TextPipeline, TextStyle, TextVertex, TextVertices,
};
use bevy_transform::{components::Transform, prelude::GlobalTransform};

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

pub fn text_system(
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Changed<Text>,
        &mut TextVertices,
        &Node,
        &Transform,
        &mut CalculatedSize,
    )>,
) {
    for (text, mut vertices, node, trans, mut size) in &mut text_query.iter() {
        let screen_position = trans.translation;
        if let Err(e) = text_pipeline.queue_text(
            text.font.clone(),
            &fonts,
            &text.value,
            text.style.font_size,
            Size::new(500., 500.),
            Vec2::new(screen_position.x(), screen_position.y()),
        ) {
            println!("Error when adding text to the queue: {:?}", e);
        }

        match text_pipeline.draw_queued(
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
    font_atlas_sets: Res<Assets<FontAtlasSet>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    meshes: Res<Assets<Mesh>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
    mut query: Query<(&mut Draw, &TextVertices, &GlobalTransform)>,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    for (mut draw, text_vertices, _) in &mut query.iter() {
        let mut text_drawer = DrawableText {
            render_resource_bindings: &mut render_resource_bindings,
            asset_render_resource_bindings: &mut asset_render_resource_bindings,
            msaa: &msaa,
            text_vertices,
            font_quad_vertex_descriptor: vertex_buffer_descriptor,
        };
        text_drawer.draw(&mut draw, &mut context).unwrap();
    }
}
