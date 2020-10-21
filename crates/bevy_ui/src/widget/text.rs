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
use bevy_transform::{prelude::GlobalTransform, components::Transform};

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
    mut text_vertices: ResMut<TextVertices>,
    mut text_query: Query<(&Text, &Transform, &mut CalculatedSize)>,
) {
    for (text, trans, mut size) in &mut text_query.iter() {
        /*
        if let Err(e) = text_pipeline.measure(&text.font, &fonts, &text.value, text.style.font_size, size.size) {
            println!("Error when measuring text: {:?}", e);
        }
        */
        println!("Queing text : {}", &text.value);
        let screen_position = trans.translation;
        if let Err(e) = text_pipeline.queue_text(
            text.font.clone(),
            &fonts,
            &text.value,
            text.style.font_size,
            size.size,
Vec2::new(screen_position.x(), screen_position.y())
        ) {
            println!("Error when adding text to the queue: {:?}", e);
        }
    }

    match text_pipeline.draw_queued(
        &fonts,
        &mut font_atlas_sets,
        &mut texture_atlases,
        &mut textures,
    ) {
        Ok(action) => match action {
            bevy_text::BrushAction::Draw(vertices) => text_vertices.set(vertices),
            bevy_text::BrushAction::Redraw => {}
        },
        Err(e) => println!("Error when drawing text: {:?}", e),
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
    text_vertices: Res<TextVertices>,
) {
    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    let mut text_drawer = TextDrawer {
        render_resource_bindings: &mut render_resource_bindings,
        asset_render_resource_bindings: &mut asset_render_resource_bindings,
        msaa: &msaa,
        text_vertices: text_vertices.borrow(),
    };

    text_drawer
        .draw(
            &mut Draw {
                is_transparent: false,
                is_visible: true,
                render_commands: vec![],
            },
            &mut context,
        )
        .unwrap();
}
