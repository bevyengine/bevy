use crate::{PositionedGlyph, TextSection};
use bevy_math::{Mat4, Vec3};
use bevy_render::pipeline::IndexFormat;
use bevy_render::{
    draw::{Draw, DrawContext, DrawError, Drawable},
    mesh,
    mesh::Mesh,
    pipeline::{PipelineSpecialization, VertexBufferLayout},
    prelude::Msaa,
    renderer::{BindGroup, RenderResourceBindings, RenderResourceId},
};
use bevy_sprite::TextureAtlasSprite;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::tracing::error;

pub struct DrawableText<'a> {
    pub render_resource_bindings: &'a mut RenderResourceBindings,
    pub global_transform: GlobalTransform,
    pub scale_factor: f32,
    pub sections: &'a [TextSection],
    pub text_glyphs: &'a Vec<PositionedGlyph>,
    pub msaa: &'a Msaa,
    pub font_quad_vertex_layout: &'a VertexBufferLayout,
    pub alignment_offset: Vec3,
}

impl<'a> Drawable for DrawableText<'a> {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError> {
        context.set_pipeline(
            draw,
            &bevy_sprite::SPRITE_SHEET_PIPELINE_HANDLE.typed(),
            &PipelineSpecialization {
                sample_count: self.msaa.samples,
                vertex_buffer_layout: self.font_quad_vertex_layout.clone(),
                ..Default::default()
            },
        )?;

        let render_resource_context = &**context.render_resource_context;

        if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_id)) = render_resource_context
            .get_asset_resource(
                &bevy_sprite::QUAD_HANDLE.typed::<Mesh>(),
                mesh::VERTEX_ATTRIBUTE_BUFFER_ID,
            )
        {
            draw.set_vertex_buffer(0, vertex_attribute_buffer_id, 0);
        } else {
            error!("Could not find vertex buffer for `bevy_sprite::QUAD_HANDLE`.")
        }

        let mut indices = 0..0;
        if let Some(RenderResourceId::Buffer(quad_index_buffer)) = render_resource_context
            .get_asset_resource(
                &bevy_sprite::QUAD_HANDLE.typed::<Mesh>(),
                mesh::INDEX_BUFFER_ASSET_INDEX,
            )
        {
            draw.set_index_buffer(quad_index_buffer, 0, IndexFormat::Uint32);
            if let Some(buffer_info) = render_resource_context.get_buffer_info(quad_index_buffer) {
                indices = 0..(buffer_info.size / 4) as u32;
            } else {
                panic!("Expected buffer type.");
            }
        }

        // set global bindings
        context.set_bind_groups_from_bindings(draw, &mut [self.render_resource_bindings])?;

        for tv in self.text_glyphs {
            context.set_asset_bind_groups(draw, &tv.atlas_info.texture_atlas)?;

            let sprite = TextureAtlasSprite {
                index: tv.atlas_info.glyph_index,
                color: self.sections[tv.section_index].style.color,
                flip_x: false,
                flip_y: false,
            };

            let transform = Mat4::from_rotation_translation(
                self.global_transform.rotation,
                self.global_transform.translation,
            ) * Mat4::from_scale(self.global_transform.scale / self.scale_factor)
                * Mat4::from_translation(
                    self.alignment_offset * self.scale_factor + tv.position.extend(0.),
                );

            let transform_buffer = context.get_uniform_buffer(&transform).unwrap();
            let sprite_buffer = context.get_uniform_buffer(&sprite).unwrap();
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
