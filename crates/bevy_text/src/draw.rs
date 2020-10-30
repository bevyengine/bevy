use crate::TextVertices;
use bevy_math::{Mat4, Vec3};
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
use bevy_sprite::TextureAtlasSprite;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

#[derive(Debug, Clone, Copy)]
pub struct TextAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: HorizontalAlign,
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub color: Color,
    pub alignment: TextAlignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            font_size: 12.0,
            alignment: TextAlignment::default(),
        }
    }
}

pub struct DrawableText<'a> {
    pub render_resource_bindings: &'a mut RenderResourceBindings,
    pub asset_render_resource_bindings: &'a mut AssetRenderResourceBindings,
    pub position: Vec3,
    pub style: &'a TextStyle,
    pub text_vertices: &'a TextVertices,
    pub msaa: &'a Msaa,
    pub font_quad_vertex_descriptor: &'a VertexBufferDescriptor,
}

impl<'a> Drawable for DrawableText<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        context.set_pipeline(
            draw,
            &bevy_sprite::SPRITE_SHEET_PIPELINE_HANDLE,
            &PipelineSpecialization {
                sample_count: self.msaa.samples,
                ..Default::default()
            },
        )?;

        let render_resource_context = &**context.render_resource_context;
        if let Some(RenderResourceId::Buffer(quad_vertex_buffer)) = render_resource_context
            .get_asset_resource(&bevy_sprite::QUAD_HANDLE, mesh::VERTEX_BUFFER_ASSET_INDEX)
        {
            draw.set_vertex_buffer(0, quad_vertex_buffer, 0);
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

        for tv in self.text_vertices.borrow() {
            let atlas_render_resource_bindings = self
                .asset_render_resource_bindings
                .get_mut(&tv.atlas_info.texture_atlas)
                .unwrap();
            context.set_bind_groups_from_bindings(draw, &mut [atlas_render_resource_bindings])?;

            let sprite = TextureAtlasSprite {
                index: tv.atlas_info.glyph_index,
                color: self.style.color,
            };

            let transform = Mat4::from_translation(self.position + tv.position.extend(0.));

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
