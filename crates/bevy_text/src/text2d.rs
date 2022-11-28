use bevy_asset::Assets;
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    event::EventReader,
    query::Changed,
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::Reflect;
use bevy_render::{
    prelude::Color,
    texture::Image,
    view::{ComputedVisibility, Visibility},
    Extract,
};
use bevy_sprite::{Anchor, ExtractedSprite, ExtractedSprites, TextureAtlas};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::HashSet;
use bevy_window::{WindowId, WindowScaleFactorChanged, Windows};

use crate::{
    Font, FontAtlasSet, FontAtlasWarning, HorizontalAlign, Text, TextError, TextLayoutInfo,
    TextPipeline, TextSettings, VerticalAlign, YAxisOrientation,
};

/// The calculated size of text drawn in 2D scene.
#[derive(Component, Default, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Text2dSize {
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
pub struct Text2dBounds {
    pub size: Vec2,
}

impl Default for Text2dBounds {
    fn default() -> Self {
        Self {
            size: Vec2::new(f32::MAX, f32::MAX),
        }
    }
}

/// The bundle of components needed to draw text in a 2D scene via a 2D `Camera2dBundle`.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
#[derive(Bundle, Clone, Debug, Default)]
pub struct Text2dBundle {
    pub text: Text,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_2d_size: Text2dSize,
    pub text_2d_bounds: Text2dBounds,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub fn extract_text2d_sprite(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Res<Windows>>,
    text2d_query: Extract<
        Query<(
            Entity,
            &ComputedVisibility,
            &Text,
            &TextLayoutInfo,
            &GlobalTransform,
            &Text2dSize,
        )>,
    >,
) {
    let scale_factor = windows.scale_factor(WindowId::primary()) as f32;

    for (entity, computed_visibility, text, text_layout_info, text_transform, calculated_size) in
        text2d_query.iter()
    {
        if !computed_visibility.is_visible() {
            continue;
        }
        let (width, height) = (calculated_size.size.x, calculated_size.size.y);

        let text_glyphs = &text_layout_info.glyphs;
        let alignment_offset = match text.alignment.vertical {
            VerticalAlign::Top => Vec3::new(0.0, -height, 0.0),
            VerticalAlign::Center => Vec3::new(0.0, -height * 0.5, 0.0),
            VerticalAlign::Bottom => Vec3::ZERO,
        } + match text.alignment.horizontal {
            HorizontalAlign::Left => Vec3::ZERO,
            HorizontalAlign::Center => Vec3::new(-width * 0.5, 0.0, 0.0),
            HorizontalAlign::Right => Vec3::new(-width, 0.0, 0.0),
        };

        let mut color = Color::WHITE;
        let mut current_section = usize::MAX;
        for text_glyph in text_glyphs {
            if text_glyph.section_index != current_section {
                color = text.sections[text_glyph.section_index]
                    .style
                    .color
                    .as_rgba_linear();
                current_section = text_glyph.section_index;
            }
            let atlas = texture_atlases
                .get(&text_glyph.atlas_info.texture_atlas)
                .unwrap();
            let handle = atlas.texture.clone_weak();
            let index = text_glyph.atlas_info.glyph_index;
            let rect = Some(atlas.textures[index]);

            let glyph_transform = Transform::from_translation(
                alignment_offset * scale_factor + text_glyph.position.extend(0.),
            );
            // NOTE: Should match `bevy_ui::render::extract_text_uinodes`
            let transform = *text_transform
                * GlobalTransform::from_scale(Vec3::splat(scale_factor.recip()))
                * glyph_transform;

            extracted_sprites.sprites.push(ExtractedSprite {
                entity,
                transform,
                color,
                rect,
                custom_size: None,
                image_handle_id: handle.id(),
                flip_x: false,
                flip_y: false,
                anchor: Anchor::Center.as_vec(),
            });
        }
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn update_text2d_layout(
    mut commands: Commands,
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    mut queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
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
        Option<&Text2dBounds>,
        &mut Text2dSize,
        Option<&mut TextLayoutInfo>,
    )>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.iter().last().is_some();
    let scale_factor = windows.scale_factor(WindowId::primary());

    for (entity, text_changed, text, maybe_bounds, mut calculated_size, text_layout_info) in
        &mut text_query
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
                Err(e @ TextError::FailedToAddGlyph(_))
                | Err(e @ TextError::ExceedMaxTextAtlases(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    calculated_size.size = Vec2::new(
                        scale_value(info.size.x, 1. / scale_factor),
                        scale_value(info.size.y, 1. / scale_factor),
                    );
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

pub fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
