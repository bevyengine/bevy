use crate::{
    BreakLineOn, Font, FontAtlasSets, FontAtlasWarning, PositionedGlyph, Text, TextError,
    TextLayoutInfo, TextPipeline, TextSettings, YAxisOrientation,
};
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
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_render::{
    prelude::Color,
    texture::Image,
    view::{InheritedVisibility, ViewVisibility, Visibility},
    Extract,
};
use bevy_sprite::{Anchor, ExtractedSprite, ExtractedSprites, TextureAtlas};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

/// The maximum width and height of text. The text will wrap according to the specified size.
/// Characters out of the bounds after wrapping will be truncated. Text is aligned according to the
/// specified [`TextAlignment`](crate::text::TextAlignment).
///
/// Note: only characters that are completely out of the bounds will be truncated, so this is not a
/// reliable limit if it is necessary to contain the text strictly in the bounds. Currently this
/// component is mainly useful for text wrapping only.
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Text2dBounds {
    /// The maximum width and height of text in logical pixels.
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
    /// Contains the text.
    pub text: Text,
    /// How the text is positioned relative to its transform.
    pub text_anchor: Anchor,
    /// The maximum width and height of the text.
    pub text_2d_bounds: Text2dBounds,
    /// The transform of the text.
    pub transform: Transform,
    /// The global transform of the text.
    pub global_transform: GlobalTransform,
    /// The visibility properties of the text.
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Contains the size of the text and its glyph's position and scale data. Generated via [`TextPipeline::queue_text`]
    pub text_layout_info: TextLayoutInfo,
}

pub fn extract_text2d_sprite(
    mut commands: Commands,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    text2d_query: Extract<
        Query<(
            &ViewVisibility,
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
    let scaling = GlobalTransform::from_scale(Vec2::splat(scale_factor.recip()).extend(1.));

    for (view_visibility, text, text_layout_info, anchor, global_transform) in text2d_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let text_anchor = -(anchor.as_vec() + 0.5);
        let alignment_translation = text_layout_info.logical_size * text_anchor;
        let transform = *global_transform
            * GlobalTransform::from_translation(alignment_translation.extend(0.))
            * scaling;
        let mut color = Color::WHITE;
        let mut current_section = usize::MAX;
        for PositionedGlyph {
            position,
            atlas_info,
            section_index,
            ..
        } in &text_layout_info.glyphs
        {
            if *section_index != current_section {
                color = text.sections[*section_index].style.color.as_rgba_linear();
                current_section = *section_index;
            }
            let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

            extracted_sprites.sprites.insert(
                commands.spawn_empty().id(),
                ExtractedSprite {
                    transform: transform * GlobalTransform::from_translation(position.extend(0.)),
                    color,
                    rect: Some(atlas.textures[atlas_info.glyph_index]),
                    custom_size: None,
                    image_handle_id: atlas.texture.id(),
                    flip_x: false,
                    flip_y: false,
                    anchor: Anchor::Center.as_vec(),
                },
            );
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
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    mut queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(Entity, Ref<Text>, Ref<Text2dBounds>, &mut TextLayoutInfo)>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.read().last().is_some();

    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    let inverse_scale_factor = scale_factor.recip();

    for (entity, text, bounds, mut text_layout_info) in &mut text_query {
        if factor_changed || text.is_changed() || bounds.is_changed() || queue.remove(&entity) {
            let text_bounds = Vec2::new(
                if text.linebreak_behavior == BreakLineOn::NoWrap {
                    f32::INFINITY
                } else {
                    scale_value(bounds.size.x, scale_factor)
                },
                scale_value(bounds.size.y, scale_factor),
            );
            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behavior,
                text_bounds,
                &mut font_atlas_sets,
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
                Ok(mut info) => {
                    info.logical_size.x = scale_value(info.logical_size.x, inverse_scale_factor);
                    info.logical_size.y = scale_value(info.logical_size.y, inverse_scale_factor);
                    *text_layout_info = info;
                }
            }
        }
    }
}

pub fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
