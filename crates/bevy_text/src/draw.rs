use crate::{glyph_brush::TextVertex, Font, FontAtlasSet};
use ab_glyph::{Glyph, PxScale, ScaleFont};
use bevy_asset::{Assets, Handle};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_render::{
    color::Color,
    draw::{Draw, DrawContext, DrawError, Drawable},
    mesh,
    pipeline::{PipelineSpecialization, VertexBufferDescriptor},
    prelude::{Msaa, Texture},
    renderer::{
        AssetRenderResourceBindings, BindGroup, BufferUsage, RenderResourceBindings,
        RenderResourceId,
    },
};
use bevy_sprite::{TextureAtlas, TextureAtlasSprite};

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            font_size: 12.0,
        }
    }
}

pub struct DrawableText<'a> {
    pub font: &'a Font,
    pub font_atlas_set: &'a FontAtlasSet,
    pub texture_atlases: &'a Assets<TextureAtlas>,
    pub render_resource_bindings: &'a mut RenderResourceBindings,
    pub asset_render_resource_bindings: &'a mut AssetRenderResourceBindings,
    pub position: Vec3,
    pub container_size: Vec2,
    pub style: &'a TextStyle,
    pub text: &'a str,
    pub msaa: &'a Msaa,
    pub font_quad_vertex_descriptor: &'a VertexBufferDescriptor,
}

impl<'a> Drawable for DrawableText<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        /*
        context.set_pipeline(
            draw,
            &bevy_sprite::SPRITE_SHEET_PIPELINE_HANDLE,
            &PipelineSpecialization {
                sample_count: self.msaa.samples,
                vertex_buffer_descriptor: self.font_quad_vertex_descriptor.clone(),
                ..Default::default()
            },
        )?;

        let render_resource_context = &**context.render_resource_context;

        if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_id)) = render_resource_context
            .get_asset_resource(&bevy_sprite::QUAD_HANDLE, mesh::VERTEX_ATTRIBUTE_BUFFER_ID)
        {
            draw.set_vertex_buffer(0, vertex_attribute_buffer_id, 0);
        } else {
            println!("could not find vertex buffer for bevy_sprite::QUAD_HANDLE")
        }

        let mut indices = 0..0;
        if let Some(RenderResourceId::Buffer(quad_index_buffer)) = render_resource_context
            .get_asset_resource(&bevy_sprite::QUAD_HANDLE, mesh::INDEX_BUFFER_ASSET_INDEX)
        {
            draw.set_index_buffer(quad_index_buffer, 0);
            if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
                indices = 0..(buffer_info.size / 4) as u32;
            } else {
                panic!("expected buffer type");
            }
        }

        // set global bindings
        context.set_bind_groups_from_bindings(draw, &mut [self.render_resource_bindings])?;

        // NOTE: this uses ab_glyph apis directly. it _might_ be a good idea to add our own layer on top
        let font = &self.font.font;
        let scale = PxScale::from(self.style.font_size);
        let scaled_font = ab_glyph::Font::as_scaled(&font, scale);
        let mut caret = self.position;
        let mut last_glyph: Option<Glyph> = None;

        // set local per-character bindings
        for character in self.text.chars() {
            if character.is_control() {
                if character == '\n' {
                    caret.set_x(self.position.x());
                    // TODO: Necessary to also calculate scaled_font.line_gap() in here?
                    caret.set_y(caret.y() - scaled_font.height());
                }
                continue;
            }

            let glyph = scaled_font.scaled_glyph(character);
            if let Some(last_glyph) = last_glyph.take() {
                caret.set_x(caret.x() + scaled_font.kern(last_glyph.id, glyph.id));
            }
            if let Some(glyph_atlas_info) = self
                .font_atlas_set
                .get_glyph_atlas_info(self.style.font_size, character)
            {
                if let Some(outlined) = scaled_font.outline_glyph(glyph.clone()) {
                    let texture_atlas = self
                        .texture_atlases
                        .get(&glyph_atlas_info.texture_atlas)
                        .unwrap();
                    let glyph_rect = texture_atlas.textures[glyph_atlas_info.char_index as usize];
                    let glyph_width = glyph_rect.width();
                    let glyph_height = glyph_rect.height();
                    let atlas_render_resource_bindings = self
                        .asset_render_resource_bindings
                        .get_mut(&glyph_atlas_info.texture_atlas)
                        .unwrap();
                    context.set_bind_groups_from_bindings(
                        draw,
                        &mut [atlas_render_resource_bindings],
                    )?;

                    let bounds = outlined.px_bounds();
                    let x = bounds.min.x + glyph_width / 2.0;
                    // the 0.5 accounts for odd-numbered heights (bump up by 1 pixel)
                    let y = -bounds.max.y + glyph_height / 2.0 - scaled_font.descent() + 0.5;
                    let transform = Mat4::from_translation(caret + Vec3::new(x, y, 0.0));
                    let sprite = TextureAtlasSprite {
                        index: glyph_atlas_info.char_index,
                        color: self.style.color,
                    };

                    let transform_buffer = context
                        .shared_buffers
                        .get_buffer(&transform, BufferUsage::UNIFORM)
                        .unwrap();
                    let sprite_buffer = context
                        .shared_buffers
                        .get_buffer(&sprite, BufferUsage::UNIFORM)
                        .unwrap();
                    let sprite_bind_group = BindGroup::build()
                        .add_binding(0, transform_buffer)
                        .add_binding(1, sprite_buffer)
                        .finish();

                    context.create_bind_group_resource(2, &sprite_bind_group)?;
                    draw.set_bind_group(2, &sprite_bind_group);
                    draw.draw_indexed(indices.clone(), 0, 0..1);
                }
            }
            caret.set_x(caret.x() + scaled_font.h_advance(glyph.id));
            last_glyph = Some(glyph);
        }
        */
        Ok(())
    }
}

pub struct TextDrawer<'a> {
    pub render_resource_bindings: &'a mut RenderResourceBindings,
    pub asset_render_resource_bindings: &'a mut AssetRenderResourceBindings,
    pub msaa: &'a Msaa,
    pub text_vertices: &'a Vec<TextVertex>,
}

impl<'a> Drawable for TextDrawer<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        context.set_pipeline(
            draw,
            bevy_sprite::SPRITE_SHEET_PIPELINE_HANDLE,
            &PipelineSpecialization {
                sample_count: self.msaa.samples,
                ..Default::default()
            },
        )?;

        let render_resource_context = &**context.render_resource_context;
        if let Some(RenderResourceId::Buffer(quad_vertex_buffer)) = render_resource_context
            .get_asset_resource(bevy_sprite::QUAD_HANDLE, mesh::VERTEX_BUFFER_ASSET_INDEX)
        {
            draw.set_vertex_buffer(0, quad_vertex_buffer, 0);
        }
        let mut indices = 0..0;
        if let Some(RenderResourceId::Buffer(quad_index_buffer)) = render_resource_context
            .get_asset_resource(bevy_sprite::QUAD_HANDLE, mesh::INDEX_BUFFER_ASSET_INDEX)
        {
            draw.set_index_buffer(quad_index_buffer, 0);
            if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
                indices = 0..(buffer_info.size / 4) as u32;
            } else {
                panic!("expected buffer type");
            }
        }

        // set global bindings
        context.set_bind_groups_from_bindings(draw, &mut [self.render_resource_bindings])?;

        for tv in self.text_vertices {
            let sprite = TextureAtlasSprite {
                index: tv.atlas_info.glyph_index,
                color: Color::WHITE,
            };
            let transform = Mat4::from_translation(Vec3::new(tv.position.x(), tv.position.y(), 0.));

            let transform_buffer = context
                .shared_buffers
                .get_buffer(&transform, BufferUsage::UNIFORM)
                .unwrap();
            let sprite_buffer = context
                .shared_buffers
                .get_buffer(&sprite, BufferUsage::UNIFORM)
                .unwrap();
            let sprite_bind_group = BindGroup::build()
                .add_binding(0, transform_buffer)
                .add_binding(1, sprite_buffer)
                .finish();
            context.create_bind_group_resource(2, &sprite_bind_group)?;
            draw.set_bind_group(2, &sprite_bind_group);
            draw.draw_indexed(indices.clone(), 0, 0..1);
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct TextVertices(Vec<TextVertex>);

impl TextVertices {
    pub fn borrow(&self) -> &Vec<TextVertex> {
        &self.0
    }

    pub fn set(&mut self, vertices: Vec<TextVertex>) {
        self.0 = vertices;
    }
}
