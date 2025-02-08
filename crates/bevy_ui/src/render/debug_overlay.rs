use crate::ui_node::ComputedNodeTarget;
use crate::CalculatedClip;
use crate::ComputedNode;
use bevy_asset::AssetId;
use bevy_color::Hsla;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::view::InheritedVisibility;
use bevy_render::Extract;
use bevy_sprite::BorderRect;
use bevy_transform::components::GlobalTransform;

use super::ExtractedUiItem;
use super::ExtractedUiNode;
use super::ExtractedUiNodes;
use super::NodeType;
use super::UiCameraMap;

/// Configuration for the UI debug overlay
#[derive(Resource)]
pub struct UiDebugOptions {
    /// Set to true to enable the UI debug overlay
    pub enabled: bool,
    /// Width of the overlay's lines in logical pixels
    pub line_width: f32,
    /// Show outlines for non-visible UI nodes
    pub show_hidden: bool,
    /// Show outlines for clipped sections of UI nodes
    pub show_clipped: bool,
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
            show_hidden: false,
            show_clipped: false,
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
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &GlobalTransform,
            &ComputedNodeTarget,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    if !debug_options.enabled {
        return;
    }

    let mut camera_mapper = camera_map.get_mapper();

    for (entity, uinode, visibility, maybe_clip, transform, computed_target) in &uinode_query {
        if !debug_options.show_hidden && !visibility.get() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(computed_target) else {
            continue;
        };

        // Extract a border box to display an outline for every UI Node in the layout
        extracted_uinodes.uinodes.push(ExtractedUiNode {
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            // Add a large number to the UI node's stack index so that the overlay is always drawn on top
            stack_index: uinode.stack_index + u32::MAX / 2,
            color: Hsla::sequential_dispersed(entity.index()).into(),
            rect: Rect {
                min: Vec2::ZERO,
                max: uinode.size,
            },
            clip: maybe_clip
                .filter(|_| !debug_options.show_clipped)
                .map(|clip| clip.clip),
            image: AssetId::default(),
            extracted_camera_entity,
            item: ExtractedUiItem::Node {
                atlas_scaling: None,
                transform: transform.compute_matrix(),
                flip_x: false,
                flip_y: false,
                border: BorderRect::all(debug_options.line_width / uinode.inverse_scale_factor()),
                border_radius: uinode.border_radius(),
                node_type: NodeType::Border,
            },
            main_entity: entity.into(),
        });
    }
}
