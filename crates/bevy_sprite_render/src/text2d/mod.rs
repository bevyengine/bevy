use crate::{
    ExtractedSlice, ExtractedSlices, ExtractedSprite, ExtractedSpriteKind, ExtractedSprites,
    ExtractedTextEffect,
};
use bevy_asset::AssetId;
use bevy_camera::visibility::ViewVisibility;
use bevy_color::LinearRgba;
use bevy_ecs::{
    entity::Entity,
    query::Has,
    system::{Commands, Query, ResMut},
};
use bevy_math::{Rect, Vec2, Vec3};
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::Extract;
use bevy_sprite::{Anchor, Text2dShadow};
use bevy_text::{
    ComputedTextBlock, PositionedGlyph, Strikethrough, StrikethroughColor, TextBackgroundColor,
    TextBounds, TextColor, TextLayoutInfo, Underline, UnderlineColor, TEXT_EFFECT_PADDING,
};
use bevy_transform::prelude::GlobalTransform;

const TEXT2D_BACKGROUND_Z_OFFSET: f32 = -0.0003;
const TEXT2D_SHADOW_Z_OFFSET: f32 = -0.0002;

/// This system extracts the sprites from the 2D text components and adds them to the
/// "render world".
pub fn extract_text2d_sprite(
    mut commands: Commands,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut extracted_slices: ResMut<ExtractedSlices>,
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
    decoration_query: Extract<
        Query<(
            &TextColor,
            Has<Strikethrough>,
            Has<Underline>,
            Option<&StrikethroughColor>,
            Option<&UnderlineColor>,
        )>,
    >,
) {
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
        let inverse_scale_factor = text_layout_info.scale_factor.recip();
        let scaling = GlobalTransform::from_scale(Vec3::new(
            inverse_scale_factor,
            -inverse_scale_factor,
            1.0,
        ));
        if !view_visibility.get() {
            continue;
        }

        let size = Vec2::new(
            text_bounds.width.unwrap_or(text_layout_info.size.x),
            text_bounds.height.unwrap_or(text_layout_info.size.y),
        );
        let top_left = (Anchor::TOP_LEFT.0 - anchor.as_vec()) * size;
        let text_transform =
            *global_transform * GlobalTransform::from_translation(top_left.extend(0.0)) * scaling;

        for run in text_layout_info.run_geometry.iter() {
            let section_entity = computed_block.entities()[run.section_index].entity;
            let Ok(text_background_color) = text_background_colors_query.get(section_entity) else {
                continue;
            };
            let render_entity = commands.spawn(TemporaryRenderEntity).id();
            let offset = run.bounds.center();
            let transform = text_transform
                * GlobalTransform::from_translation(offset.extend(TEXT2D_BACKGROUND_Z_OFFSET));
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,
                transform,
                color: text_background_color.0.into(),
                image_handle_id: AssetId::default(),
                flip_x: false,
                flip_y: true,
                text_effect: ExtractedTextEffect::default(),
                kind: ExtractedSpriteKind::Single {
                    anchor: Vec2::ZERO,
                    rect: None,
                    scaling_mode: None,
                    custom_size: Some(run.bounds.size()),
                },
            });
        }

        let shadow_effect = maybe_shadow.map(|shadow| {
            (
                LinearRgba::from(shadow.color),
                clamp_text2d_shadow_offset(shadow.offset, text_layout_info.scale_factor),
            )
        });
        let base_transform =
            *global_transform * GlobalTransform::from_translation(top_left.extend(0.0));
        let shadow_transform = shadow_effect.map(|(_, shadow_offset)| {
            base_transform
                * GlobalTransform::from_translation(
                    (shadow_offset * inverse_scale_factor).extend(0.0),
                )
                * scaling
        });
        let glyph_text_effect = shadow_effect
            .map(|(color, offset)| {
                ExtractedTextEffect::shadow(Vec2::new(offset.x, -offset.y), color)
            })
            .unwrap_or_default();
        let glyph_padding = combined_text_effect_padding(shadow_effect.map(|(_, offset)| offset));

        let mut color = LinearRgba::WHITE;
        let mut current_section = usize::MAX;
        let mut start = extracted_slices.slices.len();

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                section_index,
                ..
            },
        ) in text_layout_info.glyphs.iter().enumerate()
        {
            if *section_index != current_section {
                color = text_colors
                    .get(
                        computed_block
                            .entities()
                            .get(*section_index)
                            .map(|t| t.entity)
                            .unwrap_or(Entity::PLACEHOLDER),
                    )
                    .map(|text_color| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                current_section = *section_index;
            }

            extracted_slices.slices.push(ExtractedSlice {
                offset: *position,
                rect: glyph_padding
                    .map(|padding| expanded_effect_rect(atlas_info.rect, padding))
                    .unwrap_or(atlas_info.rect),
                size: atlas_info.rect.size() + glyph_padding.unwrap_or(Vec2::ZERO) * 2.0,
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.section_index != current_section
                    || info.atlas_info.texture != atlas_info.texture
            }) {
                let render_entity = commands.spawn(TemporaryRenderEntity).id();
                extracted_sprites.sprites.push(ExtractedSprite {
                    main_entity,
                    render_entity,
                    transform: text_transform,
                    color,
                    image_handle_id: atlas_info.texture,
                    flip_x: false,
                    flip_y: true,
                    text_effect: glyph_text_effect,
                    kind: ExtractedSpriteKind::Slices {
                        indices: start..extracted_slices.slices.len(),
                    },
                });
                start = extracted_slices.slices.len();
            }
        }

        for run in text_layout_info.run_geometry.iter() {
            let section_entity = computed_block.entities()[run.section_index].entity;
            let Ok((
                text_color,
                has_strikethrough,
                has_underline,
                maybe_strikethrough_color,
                maybe_underline_color,
            )) = decoration_query.get(section_entity)
            else {
                continue;
            };

            if has_strikethrough {
                let color = maybe_strikethrough_color
                    .map(|c| c.0)
                    .unwrap_or(text_color.0)
                    .to_linear();
                let offset = run.strikethrough_position();
                let size = run.strikethrough_size();

                if let Some(shadow) = maybe_shadow {
                    let Some(shadow_transform) = shadow_transform else {
                        continue;
                    };

                    extract_text2d_decoration(
                        &mut commands,
                        &mut extracted_sprites,
                        main_entity,
                        shadow_transform,
                        offset,
                        shadow.color.into(),
                        size,
                        TEXT2D_SHADOW_Z_OFFSET,
                    );
                }

                extract_text2d_decoration(
                    &mut commands,
                    &mut extracted_sprites,
                    main_entity,
                    text_transform,
                    offset,
                    color,
                    size,
                    0.0,
                );
            }

            if has_underline {
                let color = maybe_underline_color
                    .map(|c| c.0)
                    .unwrap_or(text_color.0)
                    .to_linear();
                let offset = run.underline_position();
                let size = run.underline_size();

                if let Some(shadow) = maybe_shadow {
                    let Some(shadow_transform) = shadow_transform else {
                        continue;
                    };

                    extract_text2d_decoration(
                        &mut commands,
                        &mut extracted_sprites,
                        main_entity,
                        shadow_transform,
                        offset,
                        shadow.color.into(),
                        size,
                        TEXT2D_SHADOW_Z_OFFSET,
                    );
                }

                extract_text2d_decoration(
                    &mut commands,
                    &mut extracted_sprites,
                    main_entity,
                    text_transform,
                    offset,
                    color,
                    size,
                    0.0,
                );
            }
        }
    }
}

fn extract_text2d_decoration(
    commands: &mut Commands,
    extracted_sprites: &mut ExtractedSprites,
    main_entity: Entity,
    transform: GlobalTransform,
    offset: Vec2,
    color: LinearRgba,
    size: Vec2,
    z_offset: f32,
) {
    let render_entity = commands.spawn(TemporaryRenderEntity).id();
    extracted_sprites.sprites.push(ExtractedSprite {
        main_entity,
        render_entity,
        transform: transform * GlobalTransform::from_translation(offset.extend(z_offset)),
        color,
        image_handle_id: AssetId::default(),
        flip_x: false,
        flip_y: false,
        text_effect: ExtractedTextEffect::default(),
        kind: ExtractedSpriteKind::Single {
            anchor: Vec2::ZERO,
            rect: None,
            scaling_mode: None,
            custom_size: Some(size),
        },
    });
}

fn clamp_text2d_shadow_offset(offset: Vec2, scale_factor: f32) -> Vec2 {
    let sampled_offset = offset * scale_factor;
    let limit = TEXT_EFFECT_PADDING as f32;
    if sampled_offset.x.abs() <= limit && sampled_offset.y.abs() <= limit {
        return sampled_offset;
    }

    sampled_offset.clamp(Vec2::splat(-limit), Vec2::splat(limit))
}

fn expanded_effect_rect(fill_rect: Rect, padding: Vec2) -> Rect {
    Rect {
        min: fill_rect.min - padding,
        max: fill_rect.max + padding,
    }
}

fn combined_text_effect_padding(shadow_offset: Option<Vec2>) -> Option<Vec2> {
    let padding = shadow_offset.map_or(Vec2::ZERO, |shadow_offset| shadow_offset.abs().ceil());
    if padding == Vec2::ZERO {
        None
    } else {
        Some(padding)
    }
}
