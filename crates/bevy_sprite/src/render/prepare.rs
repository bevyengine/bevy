use super::ExtractedSprites;
use bevy_asset::{Handle, HandleId};
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_render::prelude::Image;
use bevy_render::render_asset::RenderAssets;

const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

const QUAD_UVS: [Vec2; 4] = [
    Vec2::new(0., 1.),
    Vec2::new(1., 1.),
    Vec2::new(1., 0.),
    Vec2::new(0., 0.),
];

pub struct PreparedSprite {
    /// The main world associated entity
    pub entity: Entity,
    pub vertex_positions: [[f32; 3]; 4],
    pub vertex_uvs: [[f32; 2]; 4],
    pub image_handle_id: HandleId,
    pub color: [f32; 4],
    pub sort_key: f32,
}

#[derive(Resource, Default)]
pub struct PreparedSprites {
    pub sprites: Vec<PreparedSprite>,
}

pub fn prepare_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut prepared_sprites: ResMut<PreparedSprites>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    prepared_sprites.sprites.clear();
    for extracted_sprite in extracted_sprites.sprites.drain(..) {
        let current_image_size =
            match gpu_images.get(&Handle::weak(extracted_sprite.image_handle_id)) {
                None => continue,
                Some(img) => img.size,
            };

        // Calculate vertex data for this item

        let mut uvs = QUAD_UVS;
        if extracted_sprite.flip_x {
            uvs = [uvs[1], uvs[0], uvs[3], uvs[2]];
        }
        if extracted_sprite.flip_y {
            uvs = [uvs[3], uvs[2], uvs[1], uvs[0]];
        }

        // By default, the size of the quad is the size of the texture
        let mut quad_size = current_image_size;

        // If a rect is specified, adjust UVs and the size of the quad
        if let Some(rect) = extracted_sprite.rect {
            let rect_size = rect.size();
            for uv in &mut uvs {
                *uv = (rect.min + *uv * rect_size) / current_image_size;
            }
            quad_size = rect_size;
        }

        // Override the size if a custom one is specified
        if let Some(custom_size) = extracted_sprite.custom_size {
            quad_size = custom_size;
        }

        // Apply size and global transform
        let vertex_positions = QUAD_VERTEX_POSITIONS.map(|quad_pos| {
            extracted_sprite
                .transform
                .transform_point(((quad_pos - extracted_sprite.anchor) * quad_size).extend(0.))
                .into()
        });

        let sort_key = extracted_sprite.transform.translation().z;

        prepared_sprites.sprites.push(PreparedSprite {
            entity: extracted_sprite.entity,
            vertex_positions,
            vertex_uvs: uvs.map(Into::into),
            color: extracted_sprite.color.as_linear_rgba_f32(),
            image_handle_id: extracted_sprite.image_handle_id,
            sort_key,
        });
    }
}
