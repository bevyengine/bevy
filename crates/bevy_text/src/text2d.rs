use crate::{
    BreakLineOn, Font, FontAtlasSets, PositionedGlyph, Text, TextError, TextLayoutInfo,
    TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_asset::Assets;
use bevy_color::LinearRgba;
use bevy_ecs::{
    bundle::Bundle,
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::With,
    query::{Changed, Without},
    reflect::ReflectComponent,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_render::{
    primitives::Aabb,
    texture::Image,
    view::{InheritedVisibility, NoFrustumCulling, ViewVisibility, Visibility},
    Extract,
};
use bevy_sprite::{Anchor, ExtractedSprite, ExtractedSprites, SpriteSource, TextureAtlasLayout};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

/// The maximum width and height of text. The text will wrap according to the specified size.
/// Characters out of the bounds after wrapping will be truncated. Text is aligned according to the
/// specified [`JustifyText`](crate::text::JustifyText).
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
    ///
    /// With `Text2dBundle` the alignment field of `Text` only affects the internal alignment of a block of text and not its
    /// relative position which is controlled by the `Anchor` component.
    /// This means that for a block of text consisting of only one line that doesn't wrap, the `alignment` field will have no effect.
    pub text: Text,
    /// How the text is positioned relative to its transform.
    ///
    /// `text_anchor` does not affect the internal alignment of the block of text, only
    /// its position.
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
    /// Marks that this is a [`SpriteSource`].
    ///
    /// This is needed for visibility computation to work properly.
    pub sprite_source: SpriteSource,
}

/// This system extracts the sprites from the 2D text components and adds them to the
/// "render world".
pub fn extract_text2d_sprite(
    mut commands: Commands,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    text2d_query: Extract<
        Query<(
            Entity,
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
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);
    let scaling = GlobalTransform::from_scale(Vec2::splat(scale_factor.recip()).extend(1.));

    for (original_entity, view_visibility, text, text_layout_info, anchor, global_transform) in
        text2d_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        let text_anchor = -(anchor.as_vec() + 0.5);
        let alignment_translation = text_layout_info.logical_size * text_anchor;
        let transform = *global_transform
            * GlobalTransform::from_translation(alignment_translation.extend(0.))
            * scaling;
        let mut color = LinearRgba::WHITE;
        let mut current_section = usize::MAX;
        for PositionedGlyph {
            position,
            atlas_info,
            section_index,
            ..
        } in &text_layout_info.glyphs
        {
            if *section_index != current_section {
                color = LinearRgba::from(text.sections[*section_index].style.color);
                current_section = *section_index;
            }
            let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

            let entity = commands.spawn_empty().id();
            extracted_sprites.sprites.insert(
                entity,
                ExtractedSprite {
                    transform: transform * GlobalTransform::from_translation(position.extend(0.)),
                    color,
                    rect: Some(atlas.textures[atlas_info.glyph_index].as_rect()),
                    custom_size: None,
                    image_handle_id: atlas_info.texture.id(),
                    flip_x: false,
                    flip_y: false,
                    anchor: Anchor::Center.as_vec(),
                    original_entity: Some(original_entity),
                },
            );
        }
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the [`TextPipeline`] on insertion, then stored.
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
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
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
                text.justify,
                text.linebreak_behavior,
                text_bounds,
                &mut font_atlas_sets,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
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

/// Scales `value` by `factor`.
pub fn scale_value(value: f32, factor: f32) -> f32 {
    value * factor
}

/// System calculating and inserting an [`Aabb`] component to entities with some
/// [`TextLayoutInfo`] and [`Anchor`] components, and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`](bevy_render::view::VisibilitySystems::CalculateBounds).
pub fn calculate_bounds_text2d(
    mut commands: Commands,
    mut text_to_update_aabb: Query<
        (Entity, &TextLayoutInfo, &Anchor, Option<&mut Aabb>),
        (Changed<TextLayoutInfo>, Without<NoFrustumCulling>),
    >,
) {
    for (entity, layout_info, anchor, aabb) in &mut text_to_update_aabb {
        // `Anchor::as_vec` gives us an offset relative to the text2d bounds, by negating it and scaling
        // by the logical size we compensate the transform offset in local space to get the center.
        let center = (-anchor.as_vec() * layout_info.logical_size)
            .extend(0.0)
            .into();
        // Distance in local space from the center to the x and y limits of the text2d bounds.
        let half_extents = (layout_info.logical_size / 2.0).extend(0.0).into();
        if let Some(mut aabb) = aabb {
            *aabb = Aabb {
                center,
                half_extents,
            };
        } else {
            commands.entity(entity).try_insert(Aabb {
                center,
                half_extents,
            });
        }
    }
}

#[cfg(test)]
mod tests {

    use bevy_app::{App, Update};
    use bevy_asset::{load_internal_binary_asset, Handle};
    use bevy_ecs::{event::Events, schedule::IntoSystemConfigs};
    use bevy_utils::default;

    use super::*;

    const FIRST_TEXT: &str = "Sample text.";
    const SECOND_TEXT: &str = "Another, longer sample text.";

    fn setup() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Assets<Font>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<TextSettings>()
            .init_resource::<FontAtlasSets>()
            .init_resource::<Events<WindowScaleFactorChanged>>()
            .insert_resource(TextPipeline::default())
            .add_systems(
                Update,
                (
                    update_text2d_layout,
                    calculate_bounds_text2d.after(update_text2d_layout),
                ),
            );

        // A font is needed to ensure the text is laid out with an actual size.
        load_internal_binary_asset!(
            app,
            Handle::default(),
            "FiraMono-subset.ttf",
            |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
        );

        let entity = app
            .world_mut()
            .spawn((Text2dBundle {
                text: Text::from_section(FIRST_TEXT, default()),
                ..default()
            },))
            .id();

        (app, entity)
    }

    #[test]
    fn calculate_bounds_text2d_create_aabb() {
        let (mut app, entity) = setup();

        assert!(!app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .contains::<Aabb>());

        // Creates the AABB after text layouting.
        app.update();

        let aabb = app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Text should have an AABB");

        // Text2D AABB does not have a depth.
        assert_eq!(aabb.center.z, 0.0);
        assert_eq!(aabb.half_extents.z, 0.0);

        // AABB has an actual size.
        assert!(aabb.half_extents.x > 0.0 && aabb.half_extents.y > 0.0);
    }

    #[test]
    fn calculate_bounds_text2d_update_aabb() {
        let (mut app, entity) = setup();

        // Creates the initial AABB after text layouting.
        app.update();

        let first_aabb = *app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find initial AABB");

        let mut entity_ref = app
            .world_mut()
            .get_entity_mut(entity)
            .expect("Could not find entity");
        *entity_ref
            .get_mut::<Text>()
            .expect("Missing Text on entity") = Text::from_section(SECOND_TEXT, default());

        // Recomputes the AABB.
        app.update();

        let second_aabb = *app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find second AABB");

        // Check that the height is the same, but the width is greater.
        approx::assert_abs_diff_eq!(first_aabb.half_extents.y, second_aabb.half_extents.y);
        assert!(FIRST_TEXT.len() < SECOND_TEXT.len());
        assert!(first_aabb.half_extents.x < second_aabb.half_extents.x);
    }
}
