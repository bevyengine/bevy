use bevy_asset::AssetId;
use bevy_color::Hsla;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::system::Resource;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_render::sync_world::RenderEntity;
use bevy_render::sync_world::TemporaryRenderEntity;
use bevy_render::Extract;
use bevy_sprite::BorderRect;
use bevy_transform::components::GlobalTransform;

use crate::ComputedNode;
use crate::DefaultUiCamera;
use crate::TargetCamera;

use super::ExtractedUiItem;
use super::ExtractedUiNode;
use super::ExtractedUiNodes;
use super::NodeType;

#[derive(Resource)]
pub struct UiDebugOptions {
    pub enabled: bool,
    pub line_width: f32,
}

impl Default for UiDebugOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            line_width: 3.,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn extract_debug_overlay(
    mut commands: Commands,
    debug_options: Extract<Res<UiDebugOptions>>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    default_ui_camera: Extract<DefaultUiCamera>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &GlobalTransform,
            Option<&TargetCamera>,
        )>,
    >,
    mapping: Extract<Query<RenderEntity>>,
) {
    if !debug_options.enabled {
        return;
    }

    for (entity, uinode, transform, camera) in &uinode_query {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };

        let Ok(render_camera_entity) = mapping.get(camera_entity) else {
            continue;
        };

        extracted_uinodes.uinodes.insert(
            commands.spawn(TemporaryRenderEntity).id(),
            ExtractedUiNode {
                stack_index: uinode.stack_index + 2_147_483_647,
                color: Hsla::sequential_dispersed(entity.index()).into(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.size,
                },
                clip: None,
                image: AssetId::default(),
                camera_entity: render_camera_entity,
                item: ExtractedUiItem::Node {
                    atlas_scaling: None,
                    transform: transform.compute_matrix(),
                    flip_x: false,
                    flip_y: false,
                    border: BorderRect::square(
                        debug_options.line_width / uinode.inverse_scale_factor(),
                    ),
                    border_radius: uinode.border_radius(),
                    node_type: NodeType::Border,
                },
                main_entity: entity.into(),
            },
        );
    }
}
