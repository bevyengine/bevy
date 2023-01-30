use crate::{
    scale_value, Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo,
    TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_asset::Assets;
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    event::EventReader,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_render::{
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
#[allow(clippy::too_many_arguments)]
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
    )>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.iter().last().is_some();

    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

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
