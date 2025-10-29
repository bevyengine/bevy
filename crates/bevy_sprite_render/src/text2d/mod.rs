use crate::{
    ExtractedSlice, ExtractedSlices, ExtractedSprite, ExtractedSpriteKind, ExtractedSprites,
};
use bevy_asset::{AssetId, Assets};
use bevy_camera::visibility::ViewVisibility;
use bevy_color::LinearRgba;
use bevy_ecs::{
    entity::Entity,
    query::Has,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::prelude::*;
use bevy_math::Vec2;
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::Extract;
use bevy_sprite::{Anchor, Text2dShadow};
use bevy_text::{
    ComputedTextBlock, PositionedGlyph, Strikethrough, TextBackgroundColor, TextBounds, TextColor,
    TextLayoutInfo, Underline,
};
use bevy_transform::prelude::GlobalTransform;

/// This system extracts the sprites from the 2D text components and adds them to the
/// "render world".
pub fn extract_text2d_sprite(
    mut commands: Commands,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut extracted_slices: ResMut<ExtractedSlices>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    text2d_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &ComputedTextBlock,
            &TextLayoutInfo,
            &TextBounds,
            &Anchor,
            Option<&Text2dShadow>,
            &GlobalTransform,
        )>,
    >,
    text_colors: Extract<Query<&TextColor>>,
    text_background_colors_query: Extract<Query<&TextBackgroundColor>>,
    decoration_query: Extract<Query<(&TextColor, Has<Strikethrough>, Has<Underline>)>>,
) {
    let mut start = extracted_slices.slices.len();
    let mut end = start + 1;

    for (
        main_entity,
        view_visibility,
        computed_block,
        text_layout_info,
        text_bounds,
        anchor,
        maybe_shadow,
        global_transform,
    ) in text2d_query.iter()
    {
        let scaling = GlobalTransform::from_scale(
            Vec2::splat(text_layout_info.scale_factor.recip()).extend(1.),
        );
        if !view_visibility.get() {
            continue;
        }

        let size = Vec2::new(
            text_bounds.width.unwrap_or(text_layout_info.size.x),
            text_bounds.height.unwrap_or(text_layout_info.size.y),
        );

        let top_left = (Anchor::TOP_LEFT.0 - anchor.as_vec()) * size;

        for &(section_index, rect, _, _, _) in text_layout_info.section_geometry.iter() {
            let section_entity = computed_block.entities()[section_index].entity;
            let Ok(text_background_color) = text_background_colors_query.get(section_entity) else {
                continue;
            };
            let render_entity = commands.spawn(TemporaryRenderEntity).id();
            let offset = Vec2::new(rect.center().x, -rect.center().y);
            let transform = *global_transform
                * GlobalTransform::from_translation(top_left.extend(0.))
                * scaling
                * GlobalTransform::from_translation(offset.extend(0.));
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,
                transform,
                color: text_background_color.0.into(),
                image_handle_id: AssetId::default(),
                flip_x: false,
                flip_y: false,
                kind: ExtractedSpriteKind::Single {
                    anchor: Vec2::ZERO,
                    rect: None,
                    scaling_mode: None,
                    custom_size: Some(rect.size()),
                },
            });
        }

        if let Some(shadow) = maybe_shadow {
            let shadow_transform = *global_transform
                * GlobalTransform::from_translation((top_left + shadow.offset).extend(0.))
                * scaling;
            let color = shadow.color.into();

            for (
                i,
                PositionedGlyph {
                    position,
                    atlas_info,
                    ..
                },
            ) in text_layout_info.glyphs.iter().enumerate()
            {
                let rect = texture_atlases
                    .get(atlas_info.texture_atlas)
                    .unwrap()
                    .textures[atlas_info.location.glyph_index]
                    .as_rect();
                extracted_slices.slices.push(ExtractedSlice {
                    offset: Vec2::new(position.x, -position.y),
                    rect,
                    size: rect.size(),
                });

                if text_layout_info
                    .glyphs
                    .get(i + 1)
                    .is_none_or(|info| info.atlas_info.texture != atlas_info.texture)
                {
                    let render_entity = commands.spawn(TemporaryRenderEntity).id();
                    extracted_sprites.sprites.push(ExtractedSprite {
                        main_entity,
                        render_entity,
                        transform: shadow_transform,
                        color,
                        image_handle_id: atlas_info.texture,
                        flip_x: false,
                        flip_y: false,
                        kind: ExtractedSpriteKind::Slices {
                            indices: start..end,
                        },
                    });
                    start = end;
                }

                end += 1;
            }

            for &(section_index, rect, strikethrough_y, stroke, underline_y) in
                text_layout_info.section_geometry.iter()
            {
                let section_entity = computed_block.entities()[section_index].entity;
                let Ok((_, has_strikethrough, has_underline)) =
                    decoration_query.get(section_entity)
                else {
                    continue;
                };

                if has_strikethrough {
                    let render_entity = commands.spawn(TemporaryRenderEntity).id();
                    let offset = Vec2::new(rect.center().x, -strikethrough_y - 0.5 * stroke);
                    let transform =
                        shadow_transform * GlobalTransform::from_translation(offset.extend(0.));
                    extracted_sprites.sprites.push(ExtractedSprite {
                        main_entity,
                        render_entity,
                        transform,
                        color,
                        image_handle_id: AssetId::default(),
                        flip_x: false,
                        flip_y: false,
                        kind: ExtractedSpriteKind::Single {
                            anchor: Vec2::ZERO,
                            rect: None,
                            scaling_mode: None,
                            custom_size: Some(Vec2::new(rect.size().x, stroke)),
                        },
                    });
                }

                if has_underline {
                    let render_entity = commands.spawn(TemporaryRenderEntity).id();
                    let offset = Vec2::new(rect.center().x, -underline_y - 0.5 * stroke);
                    let transform =
                        shadow_transform * GlobalTransform::from_translation(offset.extend(0.));
                    extracted_sprites.sprites.push(ExtractedSprite {
                        main_entity,
                        render_entity,
                        transform,
                        color,
                        image_handle_id: AssetId::default(),
                        flip_x: false,
                        flip_y: false,
                        kind: ExtractedSpriteKind::Single {
                            anchor: Vec2::ZERO,
                            rect: None,
                            scaling_mode: None,
                            custom_size: Some(Vec2::new(rect.size().x, stroke)),
                        },
                    });
                }
            }
        }

        let transform =
            *global_transform * GlobalTransform::from_translation(top_left.extend(0.)) * scaling;
        let mut color = LinearRgba::WHITE;
        let mut current_span = usize::MAX;

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                span_index,
                ..
            },
        ) in text_layout_info.glyphs.iter().enumerate()
        {
            if *span_index != current_span {
                color = text_colors
                    .get(
                        computed_block
                            .entities()
                            .get(*span_index)
                            .map(|t| t.entity)
                            .unwrap_or(Entity::PLACEHOLDER),
                    )
                    .map(|text_color| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                current_span = *span_index;
            }
            let rect = texture_atlases
                .get(atlas_info.texture_atlas)
                .unwrap()
                .textures[atlas_info.location.glyph_index]
                .as_rect();
            extracted_slices.slices.push(ExtractedSlice {
                offset: Vec2::new(position.x, -position.y),
                rect,
                size: rect.size(),
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.span_index != current_span || info.atlas_info.texture != atlas_info.texture
            }) {
                let render_entity = commands.spawn(TemporaryRenderEntity).id();
                extracted_sprites.sprites.push(ExtractedSprite {
                    main_entity,
                    render_entity,
                    transform,
                    color,
                    image_handle_id: atlas_info.texture,
                    flip_x: false,
                    flip_y: false,
                    kind: ExtractedSpriteKind::Slices {
                        indices: start..end,
                    },
                });
                start = end;
            }

            end += 1;
        }

        for &(section_index, rect, strikethrough_y, stroke, underline_y) in
            text_layout_info.section_geometry.iter()
        {
            let section_entity = computed_block.entities()[section_index].entity;
            let Ok((text_color, has_strike_through, has_underline)) =
                decoration_query.get(section_entity)
            else {
                continue;
            };
            if has_strike_through {
                let render_entity = commands.spawn(TemporaryRenderEntity).id();
                let offset = Vec2::new(rect.center().x, -strikethrough_y - 0.5 * stroke);
                let transform = *global_transform
                    * GlobalTransform::from_translation(top_left.extend(0.))
                    * scaling
                    * GlobalTransform::from_translation(offset.extend(0.));
                extracted_sprites.sprites.push(ExtractedSprite {
                    main_entity,
                    render_entity,
                    transform,
                    color: text_color.0.into(),
                    image_handle_id: AssetId::default(),
                    flip_x: false,
                    flip_y: false,
                    kind: ExtractedSpriteKind::Single {
                        anchor: Vec2::ZERO,
                        rect: None,
                        scaling_mode: None,
                        custom_size: Some(Vec2::new(rect.size().x, stroke)),
                    },
                });
            }

            if has_underline {
                let render_entity = commands.spawn(TemporaryRenderEntity).id();
                let offset = Vec2::new(rect.center().x, -underline_y - 0.5 * stroke);
                let transform = *global_transform
                    * GlobalTransform::from_translation(top_left.extend(0.))
                    * scaling
                    * GlobalTransform::from_translation(offset.extend(0.));
                extracted_sprites.sprites.push(ExtractedSprite {
                    main_entity,
                    render_entity,
                    transform,
                    color: text_color.0.into(),
                    image_handle_id: AssetId::default(),
                    flip_x: false,
                    flip_y: false,
                    kind: ExtractedSpriteKind::Single {
                        anchor: Vec2::ZERO,
                        rect: None,
                        scaling_mode: None,
                        custom_size: Some(Vec2::new(rect.size().x, stroke)),
                    },
                });
            }
        }
    }
}
