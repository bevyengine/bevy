use crate::{
    scale_value, Font, FontAtlasSet, FontAtlasWarning, PositionedGlyph, Text, TextError,
    TextLayoutInfo, TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    event::EventReader,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::{Rect, Vec2};
use bevy_pbr::{AlphaMode, StandardMaterial};
use bevy_reflect::Reflect;
use bevy_render::{
    mesh::{Indices, Mesh},
    prelude::Color,
    render_resource::PrimitiveTopology,
    texture::Image,
    view::{ComputedVisibility, Visibility},
};
use bevy_sprite::TextureAtlas;
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

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

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_text3d_layout(
    mut commands: Commands,
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    mut queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Entity,
        Changed<Text>,
        &Text,
        Option<&Text3dBounds>,
        &mut Text3dSize,
        Option<&mut TextLayoutInfo>,
        Option<&Handle<Mesh>>,
        Option<&Handle<StandardMaterial>>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.iter().last().is_some();

    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    for (
        entity,
        text_changed,
        text,
        maybe_bounds,
        mut calculated_size,
        text_layout_info,
        maybe_mesh,
        maybe_material,
    ) in &mut text_query
    {
        if factor_changed || text_changed || queue.remove(&entity) {
            let text_bounds = match maybe_bounds {
                Some(bounds) => Vec2::new(
                    scale_value(bounds.size.x, scale_factor),
                    scale_value(bounds.size.y, scale_factor),
                ),
                None => Vec2::new(f32::MAX, f32::MAX),
            };

            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behaviour,
                text_bounds,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                &mut font_atlas_warning,
                YAxisOrientation::BottomToTop,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    queue.insert(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    calculated_size.size = Vec2::new(
                        scale_value(info.size.x, 1. / scale_factor),
                        scale_value(info.size.y, 1. / scale_factor),
                    );

                    if !info.glyphs.is_empty() {
                        // we assume all glyphs are on the same texture
                        // if not, we'll have to implement different materials
                        debug_assert!(info
                            .glyphs
                            .iter()
                            .zip(info.glyphs.iter().skip(1))
                            .all(|(left, right)| left.atlas_info.texture_atlas
                                == right.atlas_info.texture_atlas));
                        if let Some(atlas) =
                            texture_atlases.get(&info.glyphs[0].atlas_info.texture_atlas)
                        {
                            let new_mesh = build_mesh(&info, atlas);
                            let new_material = StandardMaterial {
                                base_color_texture: Some(atlas.texture.clone()),
                                base_color: Color::WHITE, // TODO: Get this from the text
                                alpha_mode: AlphaMode::Mask(1.0),
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

                    match text_layout_info {
                        Some(mut t) => *t = info,
                        None => {
                            commands.entity(entity).insert(info);
                        }
                    }
                }
            }
        }
    }
}

/// Build a mesh for the given text layout
fn build_mesh(info: &TextLayoutInfo, atlas: &TextureAtlas) -> Mesh {
    let mut positions = Vec::with_capacity(info.glyphs.len() * 4);
    let mut normals = Vec::with_capacity(info.glyphs.len() * 4);
    let mut uvs = Vec::with_capacity(info.glyphs.len() * 4);
    let mut indices = Vec::with_capacity(info.glyphs.len() * 6);

    for PositionedGlyph {
        position,
        size,
        atlas_info,
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
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}
