use crate::{
    Font, FontAtlasSet, FontAtlasWarning, Text, Text2dBounds, Text3dBounds, TextError,
    TextLayoutInfo, TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_asset::Assets;
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::With,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

pub fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

pub trait TextBounds: Component {
    fn size(&self) -> Vec2;
}

impl TextBounds for Text2dBounds {
    fn size(&self) -> Vec2 {
        self.size
    }
}
impl TextBounds for Text3dBounds {
    fn size(&self) -> Vec2 {
        self.size
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
pub fn update_text_layout<B: TextBounds>(
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
    mut text_query: Query<(Entity, Ref<Text>, &B, Option<&mut TextLayoutInfo>)>,
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
                scale_value(bounds.size().x, scale_factor),
                scale_value(bounds.size().y, scale_factor),
            );

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
