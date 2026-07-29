use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
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
    pub stack_index: u32,
}

/// Uinode geometries and list of geometries that changed this frame.
#[derive(Resource, Default)]
pub struct ExtractedUiGeometries {
    /// Map from extracted camera view render world entity -> UI node main world entity
    pub camera_to_uinode_map: EntityHashMap<MainEntityHashSet>,
    /// Map from main world entity -> node geometry
    pub uinode_geometries: MainEntityHashMap<ExtractedUiNodeGeometry>,
    /// Main world entities with UI node geometry that changed this frame.
    pub changed: MainEntityHashSet,
}

pub fn extract_uinode_geometry(
    mut extracted_geometries: bevy_ecs::system::ResMut<ExtractedUiGeometries>,
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
    unfiltered_geometry_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &ComputedUiTargetCamera,
            &InheritedVisibility,
            &UiGlobalTransform,
            Option<&CalculatedClip>,
        )>,
    >,

    (
        mut removed_computed_node_query,
        mut removed_computed_stack_index_query,
        mut removed_computed_ui_target_camera_query,
        mut removed_inherited_visibility_query,
        mut removed_ui_global_transform_query,
        mut removed_calculated_clip_query,
    ): (
        Extract<RemovedComponents<ComputedNode>>,
        Extract<RemovedComponents<ComputedStackIndex>>,
        Extract<RemovedComponents<ComputedUiTargetCamera>>,
        Extract<RemovedComponents<InheritedVisibility>>,
        Extract<RemovedComponents<UiGlobalTransform>>,
        Extract<RemovedComponents<CalculatedClip>>,
    ),
) {
    // Changed list is cleared each frame, then repopulated
    extracted_geometries.changed.clear();

    for entity in removed_calculated_clip_query.read() {
        let main_entity = entity.into();
        extracted_geometries.changed.insert(main_entity);
        for uinodes in extracted_geometries.uinode_geometries.values_mut() {
            if let Some(geometry) = uinodes.get_mut(&main_entity) {
                geometry.clip = None;
            }
        }
    }

    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        computed_node,
        stack_index,
        computed_target,
        inherited_visibility,
        transform,
        clip,
    ) in changed_geometry_query.iter().chain(
        removed_calculated_clip_query
            .read()
            .filter_map(|entity| unfiltered_geometry_query.get(entity).ok()),
    ) {
        let main_entity = entity.into();
        extracted_geometries.changed.insert(main_entity);

        // Find extracted camera entity, otherwise remove both camera and UI node entity.
        let Some(extracted_camera_entity) = camera_mapper.map(computed_target) else {
            extracted_geometries.uinode_geometries.remove(&main_entity);
            continue;
        };

        // Remove non-visible from all maps
        if !inherited_visibility.get() {
            extracted_geometries.uinode_geometries.remove(&main_entity);
            extracted_geometries.camera_to_uinode_map[&extracted_camera_entity]
                .remove(&main_entity);
            continue;
        }

        extracted_geometries.uinode_geometries.insert(
            main_entity,
            ExtractedUiNodeGeometry {
                computed_node: *computed_node,
                transform: transform.into(),
                clip: clip.map(|clip| clip.clip),
                atlas_scaling: None,
                flip_x: false,
                flip_y: false,
                stack_index: stack_index.0,
            },
        );
    }

    // If UI Entities are missing any of these components their retained data will be deleted from the render
    // world and they will be not be rendered:
    // - ComputedNode
    // - ComputedUiStackIndex
    // - ComputedUiTargetCamera
    // - InheritedVisibility
    // - UiGlobalTransform
    for entity in removed_computed_node_query
        .read()
        .chain(removed_computed_stack_index_query.read())
        .chain(removed_computed_ui_target_camera_query.read())
        .chain(removed_ui_global_transform_query.read())
        .chain(removed_inherited_visibility_query.read())
    {
        let main_entity = entity.into();

        match extracted_geometries.changed.entry(main_entity) {
            bevy_platform::collections::hash_set::Entry::Occupied(_) => {
                // UI node entity
            }
            bevy_platform::collections::hash_set::Entry::Vacant(_) => {}
        }

        extracted_geometries.changed.insert(main_entity);

        for uinodes in extracted_geometries.uinode_geometries.values_mut() {
            uinodes.remove(&main_entity);
        }
    }
}
