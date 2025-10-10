use crate::{Anchor, Sprite};
use bevy_asset::Assets;
use bevy_camera::primitives::Aabb;
use bevy_camera::visibility::{
    self, NoFrustumCulling, RenderLayers, Visibility, VisibilityClass, VisibleEntities,
};
use bevy_camera::Camera;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    prelude::ReflectComponent,
    query::{Changed, Without},
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_image::prelude::*;
use bevy_math::{FloatOrd, Vec2, Vec3};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{
    ComputedTextBlock, CosmicFontSystem, Font, FontAtlasSet, LineBreak, SwashCache, TextBounds,
    TextColor, TextEntities, TextError, TextFont, TextLayout, TextLayoutInfo, TextOutput,
    TextPipeline, TextRoot, TextSection, TextTarget,
};
use bevy_transform::components::Transform;
use core::any::TypeId;

/// The top-level 2D text component.
///
/// Adding `Text2d` to an entity will pull in required components for setting up 2d text.
/// [Example usage.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/text2d.rs)
///
/// The string in this component is the first 'text span' in a hierarchy of text spans that are collected into
/// a [`ComputedTextBlock`].
///
/// With `Text2d` the `justify` field of [`TextLayout`] only affects the internal alignment of a block of text and not its
/// relative position, which is controlled by the [`Anchor`] component.
/// This means that for a block of text consisting of only one line that doesn't wrap, the `justify` field will have no effect.
///
///
/// ```
/// # use bevy_asset::Handle;
/// # use bevy_color::Color;
/// # use bevy_color::palettes::basic::BLUE;
/// # use bevy_ecs::world::World;
/// # use bevy_text::{Font, Justify, TextLayout, TextFont, TextColor};
/// # use bevy_sprite::Text2d;
/// #
/// # let font_handle: Handle<Font> = Default::default();
/// # let mut world = World::default();
/// #
/// // Basic usage.
/// world.spawn(Text2d::new("hello world!"));
///
/// // With non-default style.
/// world.spawn((
///     Text2d::new("hello world!"),
///     TextFont {
///         font: font_handle.clone().into(),
///         font_size: 60.0,
///         ..Default::default()
///     },
///     TextColor(BLUE.into()),
/// ));
///
/// // With text justification.
/// world.spawn((
///     Text2d::new("hello world\nand bevy!"),
///     TextLayout::new_with_justify(Justify::Center)
/// ));
///
/// // With spans
/// world.spawn(Text2d::new("hello ")).with_children(|parent| {
///     parent.spawn(Text2d::new("world"));
///     parent.spawn((Text2d::new("!"), TextColor(BLUE.into())));
/// });
/// ```
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(
    TextFont,
    TextColor,
    Visibility,
    VisibilityClass,
    Transform,
    TextTarget,
    TextBounds,
    TextLayout,
    Anchor,
    TextSection
)]
#[component(on_add = visibility::add_visibility_class::<Sprite>)]
pub struct Text2d(pub String);

impl Text2d {
    /// Makes a new 2d text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
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

/// Adds a shadow behind `Text2d` text
///
/// Use `TextShadow` for text drawn with `bevy_ui`
#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, Clone, PartialEq)]
pub struct Text2dShadow {
    /// Shadow displacement
    /// With a value of zero the shadow will be hidden directly behind the text
    pub offset: Vec2,
    /// Color of the shadow
    pub color: Color,
}

impl Default for Text2dShadow {
    fn default() -> Self {
        Self {
            offset: Vec2::new(4., -4.),
            color: Color::BLACK,
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
pub fn update_text2d_layout(
    mut target_scale_factors: Local<Vec<(f32, RenderLayers)>>,
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    mut queue: Local<EntityHashSet>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    camera_query: Query<(&Camera, &VisibleEntities, Option<&RenderLayers>)>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut text_pipeline: ResMut<TextPipeline>,
    text_query: Query<(&Text2d, &TextFont)>,
    root_query: Query<(
        Entity,
        &TextRoot,
        Ref<TextLayout>,
        Ref<TextBounds>,
        Option<&RenderLayers>,
    )>,
    mut text_output_query: Query<(
        &mut TextLayoutInfo,
        &mut ComputedTextBlock,
        &mut TextEntities,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    target_scale_factors.clear();
    target_scale_factors.extend(
        camera_query
            .iter()
            .filter(|(_, visible_entities, _)| {
                !visible_entities.get(TypeId::of::<Sprite>()).is_empty()
            })
            .filter_map(|(camera, _, maybe_camera_mask)| {
                camera.target_scaling_factor().map(|scale_factor| {
                    (scale_factor, maybe_camera_mask.cloned().unwrap_or_default())
                })
            }),
    );

    let mut previous_scale_factor = 0.;
    let mut previous_mask = &RenderLayers::none();

    for (entity, root, block, bounds, maybe_render_layers) in root_query.iter() {
        let Ok((text_layout_info, mut computed, entities)) = text_output_query.get_mut(root.get())
        else {
            continue;
        };

        let entity_mask = maybe_render_layers.unwrap_or_default();

        let scale_factor = if entity_mask == previous_mask && 0. < previous_scale_factor {
            previous_scale_factor
        } else {
            // `Text2d` only supports generating a single text layout per Text2d entity. If a `Text2d` entity has multiple
            // render targets with different scale factors, then we use the maximum of the scale factors.
            let Some((scale_factor, mask)) = target_scale_factors
                .iter()
                .filter(|(_, camera_mask)| camera_mask.intersects(entity_mask))
                .max_by_key(|(scale_factor, _)| FloatOrd(*scale_factor))
            else {
                continue;
            };
            previous_scale_factor = *scale_factor;
            previous_mask = mask;
            *scale_factor
        };

        if scale_factor != text_layout_info.scale_factor
            || computed.needs_rerender()
            || bounds.is_changed()
            || (!queue.is_empty() && queue.remove(&entity))
        {
            let text_bounds = TextBounds {
                width: if block.linebreak == LineBreak::NoWrap {
                    None
                } else {
                    bounds.width.map(|width| width * scale_factor)
                },
                height: bounds.height.map(|height| height * scale_factor),
            };

            let spans = entities.iter().cloned().filter_map(|text_entity| {
                text_query
                    .get(text_entity)
                    .map(|(text, style)| (text_entity, text.0.as_str(), style))
                    .ok()
            });

            let text_layout_info = text_layout_info.into_inner();
            match text_pipeline.queue_text(
                text_layout_info,
                &fonts,
                spans,
                scale_factor as f64,
                &block,
                text_bounds,
                &mut font_atlas_set,
                &mut texture_atlases,
                &mut textures,
                computed.as_mut(),
                &mut font_system,
                &mut swash_cache,
                &entities.0,
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
                    text_layout_info.scale_factor = scale_factor;
                    text_layout_info.size *= scale_factor.recip();
                }
            }
        }
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with some
/// [`TextLayoutInfo`] and [`Anchor`] components, and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`](bevy_camera::visibility::VisibilitySystems::CalculateBounds).
pub fn calculate_bounds_text2d(
    mut commands: Commands,
    text_to_update_aabb: Query<(&TextLayoutInfo, &TextOutput), Changed<TextLayoutInfo>>,
    mut text_bounds_query: Query<
        (&TextBounds, Option<&mut Aabb>, &Anchor),
        Without<NoFrustumCulling>,
    >,
) {
    for (layout_info, relation) in &text_to_update_aabb {
        let Ok((text_bounds, aabb, anchor)) = text_bounds_query.get_mut(relation.0) else {
            continue;
        };

        let size = Vec2::new(
            text_bounds.width.unwrap_or(layout_info.size.x),
            text_bounds.height.unwrap_or(layout_info.size.y),
        );

        let x1 = (Anchor::TOP_LEFT.0.x - anchor.as_vec().x) * size.x;
        let x2 = (Anchor::TOP_LEFT.0.x - anchor.as_vec().x + 1.) * size.x;
        let y1 = (Anchor::TOP_LEFT.0.y - anchor.as_vec().y - 1.) * size.y;
        let y2 = (Anchor::TOP_LEFT.0.y - anchor.as_vec().y) * size.y;
        let new_aabb = Aabb::from_min_max(Vec3::new(x1, y1, 0.), Vec3::new(x2, y2, 0.));

        if let Some(mut aabb) = aabb {
            *aabb = new_aabb;
        } else {
            commands.entity(**relation).try_insert(new_aabb);
        }
    }
}

#[cfg(test)]
mod tests {

    use bevy_app::{App, Update};
    use bevy_asset::{load_internal_binary_asset, Handle};
    use bevy_camera::{ComputedCameraValues, RenderTargetInfo};
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use bevy_math::UVec2;

    use super::*;

    const FIRST_TEXT: &str = "Sample text.";
    const SECOND_TEXT: &str = "Another, longer sample text.";

    fn setup() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Assets<Font>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<TextureAtlasLayout>>()
            .init_resource::<FontAtlasSet>()
            .init_resource::<TextPipeline>()
            .init_resource::<CosmicFontSystem>()
            .init_resource::<SwashCache>()
            .add_systems(
                Update,
                (update_text2d_layout, calculate_bounds_text2d).chain(),
            );

        let mut visible_entities = VisibleEntities::default();
        visible_entities.push(Entity::PLACEHOLDER, TypeId::of::<Sprite>());

        app.world_mut().spawn((
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::splat(1000),
                        scale_factor: 1.,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            visible_entities,
        ));

        // A font is needed to ensure the text is laid out with an actual size.
        load_internal_binary_asset!(
            app,
            Handle::default(),
            "../../bevy_text/src/FiraMono-subset.ttf",
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
