use bevy_asset::Assets;
use bevy_ecs::{
    bundle::Bundle,
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::With,
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
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

use crate::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo, TextPipeline,
    TextSettings, YAxisOrientation,
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
pub struct Text2dBounds {
    pub size: Vec2,
}

impl Default for Text2dBounds {
    #[inline]
    fn default() -> Self {
        Self::UNBOUNDED
    }
}

impl Text2dBounds {
    /// Unbounded text will not be truncated or wrapped.
    pub const UNBOUNDED: Self = Self {
        size: Vec2::splat(f32::INFINITY),
    };
}

/// The bundle of components needed to draw text in a 2D scene via a 2D `Camera2dBundle`.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
#[derive(Bundle, Clone, Debug, Default)]
pub struct Text2dBundle {
    pub text: Text,
    pub text_anchor: Anchor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub text_2d_bounds: Text2dBounds,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub fn extract_text2d_sprite(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    text2d_query: Extract<
        Query<(
            Entity,
            &ComputedVisibility,
            &Text,
            &TextLayoutInfo,
            &Anchor,
            &GlobalTransform,
        )>,
    >,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor() as f32)
        .unwrap_or(1.0);

    for (entity, computed_visibility, text, text_layout_info, anchor, text_transform) in
        text2d_query.iter()
    {
        if !computed_visibility.is_visible() {
            continue;
        }

        let text_glyphs = &text_layout_info.glyphs;
        let text_anchor = anchor.as_vec() * Vec2::new(1., -1.) - 0.5;
        let alignment_offset = text_layout_info.size * text_anchor;
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

            let glyph_transform =
                Transform::from_translation((alignment_offset + text_glyph.position).extend(0.));

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
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Entity,
        Ref<Text>,
        &Text2dBounds,
        Option<&mut TextLayoutInfo>,
    )>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.iter().last().is_some();

    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    for (entity, text, bounds, text_layout_info) in &mut text_query {
        if factor_changed || text.is_changed() || queue.remove(&entity) {
            let text_bounds = Vec2::new(
                scale_value(bounds.size.x, scale_factor),
                scale_value(bounds.size.y, scale_factor),
            );

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
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => match text_layout_info {
                    Some(mut t) => *t = info,
                    None => {
                        commands.entity(entity).insert(info);
                    }
                },
            }
        }
    }
}

pub fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
