use super::ExtractedUiItem;
use super::ExtractedUiNode;
use super::ExtractedUiNodes;
use super::NodeType;
use super::UiCameraMap;
use crate::shader_flags;
use bevy_asset::AssetId;
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::Hsla;
use bevy_color::LinearRgba;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::prelude::ReflectResource;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_math::Affine2;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::Extract;
use bevy_sprite::BorderRect;
use bevy_ui::ui_transform::UiGlobalTransform;
use bevy_ui::CalculatedClip;
use bevy_ui::ComputedNode;
use bevy_ui::ComputedUiTargetCamera;
use bevy_ui::ResolvedBorderRadius;
use bevy_ui::UiStack;

/// Configuration for the UI debug overlay
///
/// Can be added as both a global `Resource` and locally as a `Component` to individual UI node entities.
/// The local component options override the global resource.
#[derive(Component, Resource, Reflect)]
#[reflect(Component, Resource)]
pub struct UiDebugOptions {
    /// Set to true to enable the UI debug overlay
    pub enabled: bool,
    /// Show outlines for the border boxes of UI nodes
    pub outline_border_box: bool,
    /// Show outlines for the padding boxes of UI nodes
    pub outline_padding_box: bool,
    /// Show outlines for the content boxes of UI nodes
    pub outline_content_box: bool,
    /// Show outlines for the scrollbar regions of UI nodes
    pub outline_scrollbars: bool,
    /// Width of the overlay's lines in logical pixels
    pub line_width: f32,
    /// Override Color for the overlay's lines
    pub line_color_override: Option<LinearRgba>,
    /// Show outlines for non-visible UI nodes
    pub show_hidden: bool,
    /// Show outlines for clipped sections of UI nodes
    pub show_clipped: bool,
    /// Draw outlines with sharp corners even if the UI nodes have border radii
    pub ignore_border_radius: bool,
}

impl UiDebugOptions {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

impl Default for UiDebugOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            line_width: 1.,
            line_color_override: None,
            show_hidden: false,
            show_clipped: false,
            ignore_border_radius: false,
            outline_border_box: true,
            outline_padding_box: false,
            outline_content_box: false,
            outline_scrollbars: false,
        }
    }
}

pub fn extract_debug_overlay(
    mut commands: Commands,
    debug_options: Extract<Res<UiDebugOptions>>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            Option<&UiDebugOptions>,
        )>,
    >,
    ui_stack: Extract<Res<UiStack>>,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, uinode, transform, visibility, maybe_clip, computed_target, debug) in &uinode_query
    {
        let debug_options = debug.unwrap_or(&debug_options);
        if !debug_options.enabled {
            continue;
        }
        if !debug_options.show_hidden && !visibility.get() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(computed_target) else {
            continue;
        };

        let color = debug_options
            .line_color_override
            .unwrap_or_else(|| Hsla::sequential_dispersed(entity.index_u32()).into());
        let z_order = (ui_stack.uinodes.len() as u32 + uinode.stack_index()) as f32;
        let border = BorderRect::all(debug_options.line_width / uinode.inverse_scale_factor());
        let transform = transform.affine();

        let mut push_outline = |rect: Rect, radius: ResolvedBorderRadius| {
            if rect.is_empty() {
                return;
            }

            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                // Keep all overlays above UI, and nudge each type slightly in Z so ordering is stable.
                z_order,
                clip: maybe_clip
                    .filter(|_| !debug_options.show_clipped)
                    .map(|clip| clip.clip),
                image: AssetId::default(),
                extracted_camera_entity,
                transform: transform * Affine2::from_translation(rect.center()),
                item: ExtractedUiItem::Node {
                    color,
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: rect.size(),
                    },
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border,
                    border_radius: radius,
                    node_type: NodeType::Border(shader_flags::BORDER_ALL),
                },
                main_entity: entity.into(),
            });
        };

        let border_box = Rect::from_center_size(Vec2::ZERO, uinode.size);

        if debug_options.outline_border_box {
            push_outline(border_box, uinode.border_radius());
        }

        if debug_options.outline_padding_box {
            let mut padding_box = border_box;
            padding_box.min += uinode.border.min_inset;
            padding_box.max -= uinode.border.max_inset;
            push_outline(padding_box, uinode.inner_radius());
        }

        if debug_options.outline_content_box {
            let mut content_box = border_box;
            let content_inset = uinode.content_inset();
            content_box.min += content_inset.min_inset;
            content_box.max -= content_inset.max_inset;
            push_outline(content_box, ResolvedBorderRadius::ZERO);
        }

        if debug_options.outline_scrollbars {
            if let Some((gutter, [thumb_min, thumb_max])) = uinode.horizontal_scrollbar() {
                push_outline(gutter, ResolvedBorderRadius::ZERO);
                push_outline(
                    Rect {
                        min: Vec2::new(thumb_min, gutter.min.y),
                        max: Vec2::new(thumb_max, gutter.max.y),
                    },
                    ResolvedBorderRadius::ZERO,
                );
            }
            if let Some((gutter, [thumb_min, thumb_max])) = uinode.vertical_scrollbar() {
                push_outline(gutter, ResolvedBorderRadius::ZERO);
                push_outline(
                    Rect {
                        min: Vec2::new(gutter.min.x, thumb_min),
                        max: Vec2::new(gutter.max.x, thumb_max),
                    },
                    ResolvedBorderRadius::ZERO,
                );
            }
        }
    }
}
