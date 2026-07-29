use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityIndexMap;
use bevy_ecs::lifecycle::RemovedComponents;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Or;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::Query;
use bevy_math::Affine2;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_render::sync_world::MainEntityHashMap;
use bevy_render::sync_world::MainEntityHashSet;
use bevy_render::Extract;
use bevy_sprite::BorderRect;
use bevy_ui::CalculatedClip;
use bevy_ui::ComputedNode;
use bevy_ui::ComputedStackIndex;
use bevy_ui::ComputedUiTargetCamera;
use bevy_ui::ResolvedBorderRadius;
use bevy_ui::UiGlobalTransform;

use crate::ExtractedGlyph;
use crate::UiCameraMap;

// pub struct ExtractedUiNode {
//     pub z_order: f32,
//     pub image: AssetId<Image>,
//     pub clip: Option<Rect>,
//     pub item: ExtractedUiItem,
//     pub transform: Affine2,
// }

// /// The type of UI node.
// /// This is used to determine how to render the UI node.
// #[derive(Clone, Copy, Debug, PartialEq)]
// pub enum NodeType {
//     Rect,
//     Inverted,
//     Border(u32), // shader flags
// }

// pub enum ExtractedUiItem {
//     Node {
//         color: LinearRgba,
//         rect: Rect,
//         atlas_scaling: Option<Vec2>,
//         flip_x: bool,
//         flip_y: bool,
//         /// Border radius of the UI node.
//         /// Ordering: top left, top right, bottom right, bottom left.
//         border_radius: ResolvedBorderRadius,
//         /// Border thickness of the UI node.
//         /// Ordering: left, top, right, bottom.
//         border: BorderRect,
//         node_type: NodeType,
//     },
//     /// A contiguous sequence of text glyphs from the same section
//     Glyphs {
//         /// The color, position, and UV rect of each glyph.
//         glyphs: Vec<ExtractedGlyph>,
//     },
// }

pub struct ExtractedUiNodeGeometry {
    pub computed_node: ComputedNode,
    pub transform: Affine2,
    pub clip: Option<Rect>,
    pub atlas_scaling: Option<Vec2>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub border_radius: ResolvedBorderRadius,
    pub border: BorderRect,
}

pub struct ExtractedGlyphLayout {
    pub glyphs: Vec<ExtractedGlyph>,
}

/// Uinode geometries and list of geometries that changed this frame.
#[derive(Resource, Default)]
pub struct ExtractedUiGeometries {
    /// map from extracted camera entity -> main world entity -> node geometry
    pub uinodes: MainEntityHashMap<MainEntityHashMap<ExtractedUiNodeGeometry>>,
    /// Main world entities with UI node geometry that changed this frame.
    pub changed: MainEntityHashSet,
}

pub fn extract_uinode_geometry(
    camera_map: Extract<UiCameraMap>,
    changed_geometry_query: Extract<
        Query<
            (
                Entity,
                &ComputedNode,
                &ComputedStackIndex,
                &ComputedUiTargetCamera,
                &InheritedVisibility,
                &UiGlobalTransform,
                Option<&CalculatedClip>,
            ),
            Or<(
                Changed<ComputedNode>,
                Changed<ComputedStackIndex>,
                Changed<ComputedUiTargetCamera>,
                Changed<InheritedVisibility>,
                Changed<UiGlobalTransform>,
                Changed<CalculatedClip>,
            )>,
        >,
    >,
    (
        mut removed_computed_node_query,
        mut removed_computed_stack_index_query,
        mut removed_computed_ui_target_camera_query,
        mut removed_ui_global_transform_query,
        mut removed_inherited_visibility_query,
        mut removed_calculated_clip_query,
    ): (
        Extract<RemovedComponents<ComputedNode>>,
        Extract<RemovedComponents<ComputedStackIndex>>,
        Extract<RemovedComponents<ComputedUiTargetCamera>>,
        Extract<RemovedComponents<UiGlobalTransform>>,
        Extract<RemovedComponents<InheritedVisibility>>,
        Extract<RemovedComponents<CalculatedClip>>,
    ),
) {
}
