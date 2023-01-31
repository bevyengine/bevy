use crate::{PositionedGlyph, Text, TextLayoutInfo, TextSection};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    query::Changed,
    reflect::ReflectComponent,
    system::{Commands, Query, Res, ResMut},
    world::Ref,
};
use bevy_math::{Rect, Vec2};
use bevy_pbr::{AlphaMode, StandardMaterial};
use bevy_reflect::Reflect;
use bevy_render::{
    mesh::{Indices, Mesh},
    prelude::Color,
    render_resource::PrimitiveTopology,
    view::{ComputedVisibility, Visibility},
};
use bevy_sprite::TextureAtlas;
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The calculated size of text drawn in 3D scene.
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Text3dSize {
    pub size: Vec2,
}

/// The maximum width and height of text. The text will wrap according to the specified size.
/// Characters out of the bounds after wrapping will be truncated. Text is aligned according to the
/// specified `TextAlignment`.
///
/// Note: only characters that are completely out of the bounds will be truncated, so this is not a
/// reliable limit if it is necessary to contain the text strictly in the bounds. Currently this
/// component is mainly useful for text wrapping only.
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Text3dBounds {
    pub size: Vec2,
}

impl Default for Text3dBounds {
    fn default() -> Self {
        Self {
            size: Vec2::new(f32::MAX, f32::MAX),
        }
    }
}

/// The bundle of components needed to draw text in a 3D scene via a 3D `Camera3dBundle`.
#[derive(Bundle, Clone, Debug, Default)]
pub struct Text3dBundle {
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_3d_size: Text3dSize,
    pub text_3d_bounds: Text3dBounds,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub fn update_text3d_mesh(
    mut commands: Commands,
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut text_query: Query<
        (
            Entity,
            Ref<Text>,
            &TextLayoutInfo,
            Option<&Handle<Mesh>>,
            Option<&Handle<StandardMaterial>>,
        ),
        Changed<TextLayoutInfo>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, text, info, maybe_mesh, maybe_material) in &mut text_query {
        if !info.glyphs.is_empty() {
            // we assume all glyphs are on the same texture
            // if not, we'll have to implement different materials
            debug_assert!(info.glyphs.iter().zip(info.glyphs.iter().skip(1)).all(
                |(left, right)| left.atlas_info.texture_atlas == right.atlas_info.texture_atlas
            ));
            if let Some(atlas) = texture_atlases.get(&info.glyphs[0].atlas_info.texture_atlas) {
                let new_mesh = build_mesh(&text.sections, &info, atlas);
                let new_material = StandardMaterial {
                    base_color_texture: Some(atlas.texture.clone()),
                    base_color: Color::WHITE,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                };
                match maybe_mesh.and_then(|handle| meshes.get_mut(handle)) {
                    Some(mesh) => {
                        *mesh = new_mesh;
                    }
                    None => {
                        let mesh = meshes.add(new_mesh);

                        commands.entity(entity).insert(mesh);
                    }
                }
                match maybe_material.and_then(|handle| materials.get_mut(handle)) {
                    Some(material) => {
                        *material = new_material;
                    }
                    None => {
                        let material = materials.add(new_material);
                        commands.entity(entity).insert(material);
                    }
                }
            }
        } else {
            commands
                .entity(entity)
                .remove::<Handle<Mesh>>()
                .remove::<Handle<StandardMaterial>>();
        }
    }
}

/// Build a mesh for the given text layout
fn build_mesh(sections: &[TextSection], info: &TextLayoutInfo, atlas: &TextureAtlas) -> Mesh {
    let mut positions = Vec::with_capacity(info.glyphs.len() * 4);
    let mut normals = Vec::with_capacity(info.glyphs.len() * 4);
    let mut uvs = Vec::with_capacity(info.glyphs.len() * 4);
    let mut indices = Vec::with_capacity(info.glyphs.len() * 6);
    let mut colors = Vec::with_capacity(info.glyphs.len() * 4);

    for PositionedGlyph {
        position,
        size,
        atlas_info,
        section_index,
        ..
    } in &info.glyphs
    {
        let start = positions.len() as u32;
        positions.extend([
            [position.x, position.y, 0.0],
            [position.x, position.y + size.y, 0.0],
            [position.x + size.x, position.y + size.y, 0.0],
            [position.x + size.x, position.y, 0.0],
        ]);

        normals.extend([
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]);

        indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);

        let Rect { min, max } = atlas.textures[atlas_info.glyph_index];
        let min = min / atlas.size;
        let max = max / atlas.size;
        uvs.extend([
            [min.x, max.y],
            [min.x, min.y],
            [max.x, min.y],
            [max.x, max.y],
        ]);

        let color = sections[*section_index].style.color.as_linear_rgba_f32();
        colors.extend([color, color, color, color]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}
