use bevy_app::App;
use bevy_asset::AssetId;
use bevy_asset::Assets;
use bevy_color::Alpha;
use bevy_color::LinearRgba;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::AnyOf;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::schedule::SystemSet;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::system::SystemParam;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_math::vec2;
use bevy_math::Mat4;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_render::sync_world::RenderEntity;
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::texture::TRANSPARENT_IMAGE_HANDLE;
use bevy_render::view::InheritedVisibility;
use bevy_render::Extract;
use bevy_render::ExtractSchedule;
use bevy_render::RenderApp;
use bevy_sprite::BorderRect;
use bevy_sprite::SpriteImageMode;
use bevy_text::ComputedTextBlock;
use bevy_text::PositionedGlyph;
use bevy_text::TextColor;
use bevy_text::TextLayoutInfo;
use bevy_transform::components::GlobalTransform;
use bevy_ui_render::box_shadow::ExtractedBoxShadow;
use bevy_ui_render::box_shadow::ExtractedBoxShadows;
use bevy_ui_render::ui_texture_slice_pipeline::ExtractedUiTextureSlice;
use bevy_ui_render::ui_texture_slice_pipeline::ExtractedUiTextureSlices;
use bevy_ui_render::ExtractedGlyph;
use bevy_ui_render::ExtractedUiItem;
use bevy_ui_render::ExtractedUiMaterialNode;
use bevy_ui_render::ExtractedUiMaterialNodes;
use bevy_ui_render::ExtractedUiNode;
use bevy_ui_render::ExtractedUiNodes;
use bevy_ui_render::NodeType;
use bevy_ui_render::UiMaterial;

use crate::prelude::MaterialNode;
use crate::widget;
use crate::widget::ImageNode;
use crate::BackgroundColor;
use crate::BorderColor;
use crate::BoxShadow;
use crate::CalculatedClip;
use crate::ComputedNode;
use crate::ComputedNodeTarget;
use crate::Display;
use crate::Node;
use crate::Outline;
use crate::ResolvedBorderRadius;
use crate::TextShadow;
use crate::Val;

#[derive(SystemParam)]
pub struct UiCameraMap<'w, 's> {
    mapping: Query<'w, 's, RenderEntity>,
}

impl<'w, 's> UiCameraMap<'w, 's> {
    /// Get the default camera and create the mapper
    pub fn get_mapper(&'w self) -> UiCameraMapper<'w, 's> {
        UiCameraMapper {
            mapping: &self.mapping,
            camera_entity: Entity::PLACEHOLDER,
            render_entity: Entity::PLACEHOLDER,
        }
    }
}

pub struct UiCameraMapper<'w, 's> {
    mapping: &'w Query<'w, 's, RenderEntity>,
    camera_entity: Entity,
    render_entity: Entity,
}

impl<'w, 's> UiCameraMapper<'w, 's> {
    /// Returns the render entity corresponding to the given `UiTargetCamera` or the default camera if `None`.
    pub fn map(&mut self, computed_target: &ComputedNodeTarget) -> Option<Entity> {
        let camera_entity = computed_target.camera;
        if self.camera_entity != camera_entity {
            let Ok(new_render_camera_entity) = self.mapping.get(camera_entity) else {
                return None;
            };
            self.render_entity = new_render_camera_entity;
            self.camera_entity = camera_entity;
        }

        Some(self.render_entity)
    }

    pub fn current_camera(&self) -> Entity {
        self.camera_entity
    }
}

pub fn add_ui_extraction_schedule(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .configure_sets(
            ExtractSchedule,
            (
                ExtractUiSystem::ExtractBoxShadows,
                ExtractUiSystem::ExtractBackgrounds,
                ExtractUiSystem::ExtractImages,
                ExtractUiSystem::ExtractTextureSlice,
                ExtractUiSystem::ExtractBorders,
                ExtractUiSystem::ExtractTextShadows,
                ExtractUiSystem::ExtractText,
                ExtractUiSystem::ExtractDebug,
            )
                .chain(),
        )
        .add_systems(
            ExtractSchedule,
            (
                extract_uinode_background_colors.in_set(ExtractUiSystem::ExtractBackgrounds),
                extract_uinode_images.in_set(ExtractUiSystem::ExtractImages),
                extract_uinode_borders.in_set(ExtractUiSystem::ExtractBorders),
                extract_text_shadows.in_set(ExtractUiSystem::ExtractTextShadows),
                extract_text_sections.in_set(ExtractUiSystem::ExtractText),
                extract_ui_texture_slices.in_set(ExtractUiSystem::ExtractTextureSlice),
                #[cfg(feature = "bevy_ui_debug")]
                debug_overlay::extract_debug_overlay.in_set(ExtractUiSystem::ExtractDebug),
            ),
        );
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ExtractUiSystem {
    ExtractBoxShadows,
    ExtractBackgrounds,
    ExtractImages,
    ExtractTextureSlice,
    ExtractBorders,
    ExtractTextShadows,
    ExtractText,
    ExtractDebug,
}

pub fn extract_uinode_background_colors(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
            &BackgroundColor,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, uinode, transform, inherited_visibility, clip, camera, background_color) in
        &uinode_query
    {
        // Skip invisible backgrounds
        if !inherited_visibility.get()
            || background_color.0.is_fully_transparent()
            || uinode.is_empty()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        extracted_uinodes.uinodes.push(ExtractedUiNode {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            stack_index: uinode.stack_index,
            color: background_color.0.into(),
            rect: Rect {
                min: Vec2::ZERO,
                max: uinode.size,
            },
            clip: clip.map(|clip| clip.clip),
            image: AssetId::default(),
            extracted_camera_entity,
            item: ExtractedUiItem::Node {
                atlas_scaling: None,
                transform: transform.compute_matrix(),
                flip_x: false,
                flip_y: false,
                border: uinode.border(),
                border_radius: uinode.border_radius().into(),
                node_type: NodeType::Rect,
            },
            main_entity: entity.into(),
        });
    }
}

pub fn extract_uinode_images(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
            &ImageNode,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();
    for (entity, uinode, transform, inherited_visibility, clip, camera, image) in &uinode_query {
        // Skip invisible images
        if !inherited_visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
            || image.image_mode.uses_slices()
            || uinode.is_empty()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&texture_atlases))
            .map(|r| r.as_rect());

        let mut rect = match (atlas_rect, image.rect) {
            (None, None) => Rect {
                min: Vec2::ZERO,
                max: uinode.size,
            },
            (None, Some(image_rect)) => image_rect,
            (Some(atlas_rect), None) => atlas_rect,
            (Some(atlas_rect), Some(mut image_rect)) => {
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                image_rect
            }
        };

        let atlas_scaling = if atlas_rect.is_some() || image.rect.is_some() {
            let atlas_scaling = uinode.size() / rect.size();
            rect.min *= atlas_scaling;
            rect.max *= atlas_scaling;
            Some(atlas_scaling)
        } else {
            None
        };

        extracted_uinodes.uinodes.push(ExtractedUiNode {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            stack_index: uinode.stack_index,
            color: image.color.into(),
            rect,
            clip: clip.map(|clip| clip.clip),
            image: image.image.id(),
            extracted_camera_entity,
            item: ExtractedUiItem::Node {
                atlas_scaling,
                transform: transform.compute_matrix(),
                flip_x: image.flip_x,
                flip_y: image.flip_y,
                border: uinode.border,
                border_radius: uinode.border_radius.into(),
                node_type: NodeType::Rect,
            },
            main_entity: entity.into(),
        });
    }
}

pub fn extract_uinode_borders(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
            AnyOf<(&BorderColor, &Outline)>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let image = AssetId::<Image>::default();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        node,
        computed_node,
        global_transform,
        inherited_visibility,
        maybe_clip,
        camera,
        (maybe_border_color, maybe_outline),
    ) in &uinode_query
    {
        // Skip invisible borders and removed nodes
        if !inherited_visibility.get() || node.display == Display::None {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        // Don't extract borders with zero width along all edges
        if computed_node.border() != BorderRect::ZERO {
            if let Some(border_color) = maybe_border_color.filter(|bc| !bc.0.is_fully_transparent())
            {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    stack_index: computed_node.stack_index,
                    color: border_color.0.into(),
                    rect: Rect {
                        max: computed_node.size(),
                        ..Default::default()
                    },
                    image,
                    clip: maybe_clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    item: ExtractedUiItem::Node {
                        atlas_scaling: None,
                        transform: global_transform.compute_matrix(),
                        flip_x: false,
                        flip_y: false,
                        border: computed_node.border(),
                        border_radius: computed_node.border_radius().into(),
                        node_type: NodeType::Border,
                    },
                    main_entity: entity.into(),
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                });
            }
        }

        if computed_node.outline_width() <= 0. {
            continue;
        }

        if let Some(outline) = maybe_outline.filter(|outline| !outline.color.is_fully_transparent())
        {
            let outline_size = computed_node.outlined_node_size();
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                stack_index: computed_node.stack_index,
                color: outline.color.into(),
                rect: Rect {
                    max: outline_size,
                    ..Default::default()
                },
                image,
                clip: maybe_clip.map(|clip| clip.clip),
                extracted_camera_entity,
                item: ExtractedUiItem::Node {
                    transform: global_transform.compute_matrix(),
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border: BorderRect::all(computed_node.outline_width()),
                    border_radius: computed_node.outline_radius().into(),
                    node_type: NodeType::Border,
                },
                main_entity: entity.into(),
            });
        }
    }
}

pub fn extract_text_sections(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
            &ComputedTextBlock,
            &TextLayoutInfo,
        )>,
    >,
    text_styles: Extract<Query<&TextColor>>,
    camera_map: Extract<UiCameraMap>,
) {
    let mut start = extracted_uinodes.glyphs.len();
    let mut end = start + 1;

    let mut camera_mapper = camera_map.get_mapper();
    for (
        entity,
        uinode,
        global_transform,
        inherited_visibility,
        clip,
        camera,
        computed_block,
        text_layout_info,
    ) in &uinode_query
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let transform = global_transform.affine()
            * bevy_math::Affine3A::from_translation((-0.5 * uinode.size()).extend(0.));

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
            let rect = texture_atlases
                .get(&atlas_info.texture_atlas)
                .unwrap()
                .textures[atlas_info.location.glyph_index]
                .as_rect();
            extracted_uinodes.glyphs.push(ExtractedGlyph {
                transform: transform * Mat4::from_translation(position.extend(0.)),
                rect,
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.span_index != *span_index || info.atlas_info.texture != atlas_info.texture
            }) {
                let color = text_styles
                    .get(
                        computed_block
                            .entities()
                            .get(*span_index)
                            .map(|t| t.entity)
                            .unwrap_or(Entity::PLACEHOLDER),
                    )
                    .map(|text_color| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    stack_index: uinode.stack_index,
                    color,
                    image: atlas_info.texture.id(),
                    clip: clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    rect,
                    item: ExtractedUiItem::Glyphs { range: start..end },
                    main_entity: entity.into(),
                });
                start = end;
            }

            end += 1;
        }
    }
}

pub fn extract_text_shadows(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedNodeTarget,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &TextLayoutInfo,
            &TextShadow,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut start = extracted_uinodes.glyphs.len();
    let mut end = start + 1;

    let mut camera_mapper = camera_map.get_mapper();
    for (
        entity,
        uinode,
        target,
        global_transform,
        inherited_visibility,
        clip,
        text_layout_info,
        shadow,
    ) in &uinode_query
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target) else {
            continue;
        };

        let transform = global_transform.affine()
            * Mat4::from_translation(
                (-0.5 * uinode.size() + shadow.offset / uinode.inverse_scale_factor()).extend(0.),
            );

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
            let rect = texture_atlases
                .get(&atlas_info.texture_atlas)
                .unwrap()
                .textures[atlas_info.location.glyph_index]
                .as_rect();
            extracted_uinodes.glyphs.push(ExtractedGlyph {
                transform: transform * Mat4::from_translation(position.extend(0.)),
                rect,
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.span_index != *span_index || info.atlas_info.texture != atlas_info.texture
            }) {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    stack_index: uinode.stack_index,
                    color: shadow.color.into(),
                    image: atlas_info.texture.id(),
                    clip: clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    rect,
                    item: ExtractedUiItem::Glyphs { range: start..end },
                    main_entity: entity.into(),
                });
                start = end;
            }

            end += 1;
        }
    }
}

pub fn extract_shadows(
    mut commands: Commands,
    mut extracted_box_shadows: ResMut<ExtractedBoxShadows>,
    box_shadow_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            &BoxShadow,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut mapping = camera_map.get_mapper();

    for (entity, uinode, transform, visibility, box_shadow, clip, camera) in &box_shadow_query {
        // Skip if no visible shadows
        if !visibility.get() || box_shadow.is_empty() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = mapping.map(camera) else {
            continue;
        };

        let ui_physical_viewport_size = camera.physical_size.as_vec2();

        let scale_factor = uinode.inverse_scale_factor.recip();

        for drop_shadow in box_shadow.iter() {
            if drop_shadow.color.is_fully_transparent() {
                continue;
            }

            let resolve_val = |val, base, scale_factor| match val {
                Val::Auto => 0.,
                Val::Px(px) => px * scale_factor,
                Val::Percent(percent) => percent / 100. * base,
                Val::Vw(percent) => percent / 100. * ui_physical_viewport_size.x,
                Val::Vh(percent) => percent / 100. * ui_physical_viewport_size.y,
                Val::VMin(percent) => percent / 100. * ui_physical_viewport_size.min_element(),
                Val::VMax(percent) => percent / 100. * ui_physical_viewport_size.max_element(),
            };

            let spread_x = resolve_val(drop_shadow.spread_radius, uinode.size().x, scale_factor);
            let spread_ratio = (spread_x + uinode.size().x) / uinode.size().x;

            let spread = vec2(spread_x, uinode.size().y * spread_ratio - uinode.size().y);

            let blur_radius = resolve_val(drop_shadow.blur_radius, uinode.size().x, scale_factor);
            let offset = vec2(
                resolve_val(drop_shadow.x_offset, uinode.size().x, scale_factor),
                resolve_val(drop_shadow.y_offset, uinode.size().y, scale_factor),
            );

            let shadow_size = uinode.size() + spread;
            if shadow_size.cmple(Vec2::ZERO).any() {
                continue;
            }

            let radius = ResolvedBorderRadius {
                top_left: uinode.border_radius.top_left * spread_ratio,
                top_right: uinode.border_radius.top_right * spread_ratio,
                bottom_left: uinode.border_radius.bottom_left * spread_ratio,
                bottom_right: uinode.border_radius.bottom_right * spread_ratio,
            };

            extracted_box_shadows.box_shadows.push(ExtractedBoxShadow {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                stack_index: uinode.stack_index,
                transform: transform.compute_matrix() * Mat4::from_translation(offset.extend(0.)),
                color: drop_shadow.color.into(),
                bounds: shadow_size + 6. * blur_radius,
                clip: clip.map(|clip| clip.clip),
                extracted_camera_entity,
                radius: radius.into(),
                blur_radius,
                size: shadow_size,
                main_entity: entity.into(),
            });
        }
    }
}

pub fn extract_ui_material_nodes<M: UiMaterial>(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    materials: Extract<Res<Assets<M>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &MaterialNode<M>,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, computed_node, transform, handle, inherited_visibility, clip, camera) in
        uinode_query.iter()
    {
        // skip invisible nodes
        if !inherited_visibility.get() || computed_node.is_empty() {
            continue;
        }

        // Skip loading materials
        if !materials.contains(handle) {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        extracted_uinodes.uinodes.push(ExtractedUiMaterialNode {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            stack_index: computed_node.stack_index,
            transform: transform.compute_matrix(),
            material: handle.id(),
            rect: Rect {
                min: Vec2::ZERO,
                max: computed_node.size(),
            },
            border: computed_node.border(),
            border_radius: computed_node.border_radius.into(),
            clip: clip.map(|clip| clip.clip),
            extracted_camera_entity,
            main_entity: entity.into(),
        });
    }
}

pub fn extract_ui_texture_slices(
    mut commands: Commands,
    mut extracted_ui_slicers: ResMut<ExtractedUiTextureSlices>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    slicers_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
            &ImageNode,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, uinode, transform, inherited_visibility, clip, camera, image) in &slicers_query {
        // Skip invisible images
        if !inherited_visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
        {
            continue;
        }

        let image_scale_mode = match image.image_mode.clone() {
            widget::NodeImageMode::Sliced(texture_slicer) => {
                SpriteImageMode::Sliced(texture_slicer)
            }
            widget::NodeImageMode::Tiled {
                tile_x,
                tile_y,
                stretch_value,
            } => SpriteImageMode::Tiled {
                tile_x,
                tile_y,
                stretch_value,
            },
            _ => continue,
        };

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&texture_atlases))
            .map(|r| r.as_rect());

        let atlas_rect = match (atlas_rect, image.rect) {
            (None, None) => None,
            (None, Some(image_rect)) => Some(image_rect),
            (Some(atlas_rect), None) => Some(atlas_rect),
            (Some(atlas_rect), Some(mut image_rect)) => {
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                Some(image_rect)
            }
        };

        extracted_ui_slicers.slices.push(ExtractedUiTextureSlice {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            stack_index: uinode.stack_index,
            transform: transform.compute_matrix(),
            color: image.color.into(),
            rect: Rect {
                min: Vec2::ZERO,
                max: uinode.size,
            },
            clip: clip.map(|clip| clip.clip),
            image: image.image.id(),
            extracted_camera_entity,
            image_scale_mode,
            atlas_rect,
            flip_x: image.flip_x,
            flip_y: image.flip_y,
            inverse_scale_factor: uinode.inverse_scale_factor,
            main_entity: entity.into(),
        });
    }
}
