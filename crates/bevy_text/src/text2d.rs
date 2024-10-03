use crate::pipeline::CosmicFontSystem;
use crate::{
    ComputedTextBlock, Font, FontAtlasSets, LineBreak, PositionedGlyph, SwashCache, TextBlock,
    TextBlocks, TextBounds, TextError, TextLayoutInfo, TextPipeline, TextSpanAccess, TextStyle,
    YAxisOrientation,
};
use bevy_asset::Assets;
use bevy_color::LinearRgba;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    entity::Entity,
    event::EventReader,
    prelude::{ReflectComponent, With},
    query::{Changed, Without},
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::view::Visibility;
use bevy_render::{
    primitives::Aabb,
    texture::Image,
    view::{NoFrustumCulling, ViewVisibility},
    Extract,
};
use bevy_sprite::{Anchor, ExtractedSprite, ExtractedSprites, SpriteSource, TextureAtlasLayout};
use bevy_transform::components::Transform;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};

/// The top-level 2D text component.
///
/// Adding `Text2d` to an entity will pull in required components for setting up 2d text.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
///
/// The string in this component is the first 'text span' in a hierarchy of text spans that are collected into
/// a [`TextBlock`]. See [`TextSpan2d`] for the component used by children of entities with [`Text2d`].
///
/// With `Text2d` the `justify` field of [`TextBlock`] only affects the internal alignment of a block of text and not its
/// relative position, which is controlled by the [`Anchor`] component.
/// This means that for a block of text consisting of only one line that doesn't wrap, the `justify` field will have no effect.
///
/*
```
# use bevy_asset::Handle;
# use bevy_color::Color;
# use bevy_color::palettes::basic::BLUE;
# use bevy_ecs::World;
# use bevy_text::{Font, JustifyText, Text2d, TextBlock, TextStyle};
#
# let font_handle: Handle<Font> = Default::default();
# let mut world = World::default();
#
// Basic usage.
world.spawn(Text2d::new("hello world!"));

// With non-default style.
world.spawn((
    Text2d::new("hello world!"),
    TextStyle {
        font: font_handle.clone().into(),
        font_size: 60.0,
        color: BLUE.into(),
    }
));

// With text justification.
world.spawn((
    Text2d::new("hello world\nand bevy!"),
    TextBlock::new_with_justify(JustifyText::Center)
));
```
*/
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(
    TextBlock,
    TextStyle,
    TextBounds,
    Anchor,
    SpriteSource,
    Visibility,
    Transform
)]
pub struct Text2d(pub String);

impl Text2d {
    /// Makes a new 2d text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl TextSpanAccess for Text2d {
    fn read_span(&self) -> &str {
        self.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut *self
    }
}

impl From<&str> for Text2d {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for Text2d {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// A span of 2d text in a tree of spans under an entity with [`Text2d`].
///
/// Spans are collected in hierarchy traversal order into a [`ComputedTextBlock`] for layout.
///
/*
```
# use bevy_asset::Handle;
# use bevy_color::Color;
# use bevy_color::palettes::basic::{RED, BLUE};
# use bevy_ecs::World;
# use bevy_text::{Font, Text2d, TextSpan2d, TextStyle, TextSection};
#
# let font_handle: Handle<Font> = Default::default();
# let mut world = World::default();
#
world.spawn((
    Text2d::new("Hello, "),
    TextStyle {
        font: font_handle.clone().into(),
        font_size: 60.0,
        color: BLUE.into(),
    }
))
.with_child((
    TextSpan2d::new("World!"),
    TextStyle {
        font: font_handle.into(),
        font_size: 60.0,
        color: RED.into(),
    }
));
```
*/
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(TextStyle, Visibility(visibility_hidden), Transform)]
pub struct TextSpan2d(pub String);

impl TextSpanAccess for TextSpan2d {
    fn read_span(&self) -> &str {
        self.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut *self
    }
}

fn visibility_hidden() -> Visibility {
    Visibility::Hidden
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
            &ComputedTextBlock,
            &TextLayoutInfo,
            &Anchor,
            &GlobalTransform,
        )>,
    >,
    text_styles: Extract<Query<&TextStyle>>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);
    let scaling = GlobalTransform::from_scale(Vec2::splat(scale_factor.recip()).extend(1.));

    for (
        original_entity,
        view_visibility,
        computed_block,
        text_layout_info,
        anchor,
        global_transform,
    ) in text2d_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        let text_anchor = -(anchor.as_vec() + 0.5);
        let alignment_translation = text_layout_info.size * text_anchor;
        let transform = *global_transform
            * GlobalTransform::from_translation(alignment_translation.extend(0.))
            * scaling;
        let mut color = LinearRgba::WHITE;
        let mut current_span = usize::MAX;
        for PositionedGlyph {
            position,
            atlas_info,
            span_index,
            ..
        } in &text_layout_info.glyphs
        {
            if *span_index != current_span {
                color = text_styles
                    .get(
                        computed_block
                            .entities()
                            .get(*span_index)
                            .map(|t| t.entity)
                            .unwrap_or(Entity::PLACEHOLDER),
                    )
                    .map(|style| LinearRgba::from(style.color))
                    .unwrap_or_default();
                current_span = *span_index;
            }
            let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

            let entity = commands.spawn_empty().id();
            extracted_sprites.sprites.insert(
                entity,
                ExtractedSprite {
                    transform: transform * GlobalTransform::from_translation(position.extend(0.)),
                    color,
                    rect: Some(atlas.textures[atlas_info.location.glyph_index].as_rect()),
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
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Entity,
        Ref<TextBlock>,
        Ref<TextBounds>,
        &mut TextLayoutInfo,
        &mut ComputedTextBlock,
    )>,
    mut blocks: TextBlocks<Text2d, TextSpan2d>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.read().last().is_some();

    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    let inverse_scale_factor = scale_factor.recip();

    for (entity, block, bounds, text_layout_info, mut computed) in &mut text_query {
        if factor_changed
            || computed.needs_rerender()
            || bounds.is_changed()
            || queue.remove(&entity)
        {
            let text_bounds = TextBounds {
                width: if block.linebreak == LineBreak::NoWrap {
                    None
                } else {
                    bounds.width.map(|width| scale_value(width, scale_factor))
                },
                height: bounds
                    .height
                    .map(|height| scale_value(height, scale_factor)),
            };

            let text_layout_info = text_layout_info.into_inner();
            match text_pipeline.queue_text(
                text_layout_info,
                &fonts,
                blocks.iter(entity),
                scale_factor.into(),
                &block,
                text_bounds,
                &mut font_atlas_sets,
                &mut texture_atlases,
                &mut textures,
                YAxisOrientation::BottomToTop,
                computed.as_mut(),
                &mut font_system,
                &mut swash_cache,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    queue.insert(entity);
                }
                Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(()) => {
                    text_layout_info.size.x =
                        scale_value(text_layout_info.size.x, inverse_scale_factor);
                    text_layout_info.size.y =
                        scale_value(text_layout_info.size.y, inverse_scale_factor);
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
        let center = (-anchor.as_vec() * layout_info.size).extend(0.0).into();
        // Distance in local space from the center to the x and y limits of the text2d bounds.
        let half_extents = (layout_info.size / 2.0).extend(0.0).into();
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

    use crate::{detect_text_needs_rerender, TextIterScratch};

    use super::*;

    const FIRST_TEXT: &str = "Sample text.";
    const SECOND_TEXT: &str = "Another, longer sample text.";

    fn setup() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Assets<Font>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<FontAtlasSets>()
            .init_resource::<Events<WindowScaleFactorChanged>>()
            .init_resource::<TextPipeline>()
            .init_resource::<CosmicFontSystem>()
            .init_resource::<SwashCache>()
            .init_resource::<TextIterScratch>()
            .add_systems(
                Update,
                (
                    detect_text_needs_rerender::<Text2d, TextSpan2d>,
                    update_text2d_layout,
                    calculate_bounds_text2d,
                )
                    .chain(),
            );

        // A font is needed to ensure the text is laid out with an actual size.
        load_internal_binary_asset!(
            app,
            Handle::default(),
            "FiraMono-subset.ttf",
            |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
        );

        let entity = app.world_mut().spawn(Text2d::new(FIRST_TEXT)).id();

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
            .get_mut::<Text2d>()
            .expect("Missing Text2d on entity") = Text2d::new(SECOND_TEXT);

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
