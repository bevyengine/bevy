use crate::{PositionedGlyph, Text, TextLayoutInfo, TextSection};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::{Commands, Query, Res, ResMut},
};
use bevy_hierarchy::{BuildChildren, Children};
use bevy_math::{Rect, Vec2};
use bevy_pbr::{AlphaMode, StandardMaterial};
use bevy_reflect::Reflect;
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
    view::{ComputedVisibility, Visibility, VisibilityBundle},
};
use bevy_sprite::{Anchor, TextureAtlas};
use bevy_transform::{
    prelude::{GlobalTransform, Transform},
    TransformBundle,
};

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
    pub anchor: Anchor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_3d_bounds: Text3dBounds,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

/// Update the mesh and standard material for the entities spawned with a [`Text3dBundle`]
#[allow(clippy::type_complexity)]
pub fn update_text3d_mesh(
    mut commands: Commands,
    texture_atlases: Res<Assets<TextureAtlas>>,
    text_query: Query<
        (Entity, &Text, &Anchor, &TextLayoutInfo, Option<&Children>),
        (Changed<TextLayoutInfo>, With<Text3dBounds>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // TODO: This currently creates a child node for each unique atlas texture.
    // multiple atlas textures can be generated when a user uses multiple fonts (e.g. a normal and a bold font) in a single `Text`
    // currently we cannot create a single material with multiple textures, this might be fixed in https://github.com/bevyengine/bevy/pull/6842

    for (entity, text, anchor, info, maybe_children) in text_query.iter() {
        // if we have no glyphs, remove the children to save on resources
        if info.glyphs.is_empty() {
            if let Some(children) = maybe_children {
                for child in children.iter() {
                    commands.entity(*child).despawn();
                }
            }
            continue;
        }

        // When rendering a text with multiple segments, each segment could be a different atlas entry
        // we'll need to get all the atlasses and references and sort + dedup them
        let mut unique_atlasses = info
            .glyphs
            .iter()
            .map(|g| &g.atlas_info.texture_atlas)
            .collect::<Vec<_>>();
        unique_atlasses.sort();
        unique_atlasses.dedup();

        let text_anchor = anchor.as_vec() * Vec2::new(1., -1.) - 0.5;
        let alignment_offset = info.size * text_anchor;

        let mut unique_atlasses = info
            .glyphs
            .iter()
            .map(|g| &g.atlas_info.texture_atlas)
            .collect::<Vec<_>>();
        unique_atlasses.sort();
        unique_atlasses.dedup();

        // Create an iterator over the meshes and materials for each unique atlas entry
        let mut meshes_and_materials = unique_atlasses
            .into_iter()
            .map(|handle| {
                let (mesh, material) = build_mesh_and_material(
                    &text.sections,
                    info,
                    alignment_offset,
                    handle,
                    texture_atlases.get(handle).expect("Atlas does not exist"),
                );
                let mesh = meshes.add(mesh);
                let material = materials.add(material);
                (mesh, material)
            })
            .peekable();
        // Create an iterator over the children
        let mut children = maybe_children.into_iter().flatten().peekable();

        // re-use existing children
        while meshes_and_materials.peek().is_some() && children.peek().is_some() {
            // these unwraps are okay because we checked for `.is_some()`
            let (mesh, material) = meshes_and_materials.next().unwrap();
            let child = children.next().unwrap();
            commands.entity(*child).insert((mesh, material));
        }
        // create new children
        for (mesh, material) in meshes_and_materials {
            let child_id = commands
                .spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    mesh,
                    material,
                ))
                .id();
            commands.entity(entity).add_child(child_id);
        }
        // clean up old children that no longer have a texture
        for child in children {
            commands.entity(*child).despawn();
        }
    }
}

/// Build a mesh for the given text layout
fn build_mesh_and_material(
    sections: &[TextSection],
    info: &TextLayoutInfo,
    alignment_offset: Vec2,
    atlas_handle: &Handle<TextureAtlas>,
    atlas: &TextureAtlas,
) -> (Mesh, StandardMaterial) {
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
    } in info
        .glyphs
        .iter()
        .filter(|g| &g.atlas_info.texture_atlas == atlas_handle)
    {
        // build a quad for every single character, UV-mapped to the texture in the atlas
        let start = positions.len() as u32;
        let position = *position + alignment_offset;

        // the position from the glyph is in the center, so make a quad from `position-half_size` to `position+half_size`
        let half_size = *size / 2.;
        let tl = position - half_size; // top left
        let br = position + half_size; // bottom right
        positions.extend([
            [tl.x, tl.y, 0.0],
            [tl.x, br.y, 0.0],
            [br.x, br.y, 0.0],
            [br.x, tl.y, 0.0],
        ]);

        normals.extend([
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]);

        indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);

        let Rect { min, max } = atlas.textures[atlas_info.glyph_index];
        // this rect is the actual pixels, but we need the 0.0 .. 1.0 range, so divide by the atlas size
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

    let new_material = StandardMaterial {
        base_color_texture: Some(atlas.texture.clone()),
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    };
    (mesh, new_material)
}
