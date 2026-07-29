use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::entity::EntityHashSet;
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
    pub extracted_camera: Entity,
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
    pub extracted_camera_to_main_uinode_map: EntityHashMap<MainEntityHashSet>,
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
        // `CalculatedClip` is optional, also update geometry if it is removed.
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
            extracted_geometries
                .extracted_camera_to_main_uinode_map
                .get_mut(&extracted_camera_entity)
                .map(|main_entities| main_entities.remove(&main_entity));
            continue;
        }

        let uinode_geometry = ExtractedUiNodeGeometry {
            extracted_camera: extracted_camera_entity,
            computed_node: *computed_node,
            transform: transform.into(),
            clip: clip.map(|clip| clip.clip),
            atlas_scaling: None,
            flip_x: false,
            flip_y: false,
            stack_index: stack_index.0,
        };

        let ExtractedUiGeometries {
            extracted_camera_to_main_uinode_map,
            uinode_geometries,
            ..
        } = &mut *extracted_geometries;

        match uinode_geometries.entry(main_entity) {
            bevy_platform::collections::hash_map::Entry::Occupied(mut entry) => {
                let old_extracted_camera = entry.get().extracted_camera;
                if old_extracted_camera != extracted_camera_entity {
                    extracted_camera_to_main_uinode_map
                        .get_mut(&old_extracted_camera)
                        .map(|main_entities| main_entities.remove(&main_entity));
                    extracted_camera_to_main_uinode_map
                        .entry(extracted_camera_entity)
                        .or_default()
                        .insert(main_entity);
                }
                entry.insert(uinode_geometry);
            }
            bevy_platform::collections::hash_map::Entry::Vacant(entry) => {
                extracted_camera_to_main_uinode_map
                    .entry(extracted_camera_entity)
                    .or_default()
                    .insert(main_entity);
                entry.insert(uinode_geometry);
            }
        }
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

        // If already in changed, entity passed the geometry query so the component must have been removed and reinserted in the same frame, so do nothing.
        if extracted_geometries.changed.insert(main_entity) {
            // Failed the geometry queries, does not have at least one of the components (if reinserted would be `Changed`).
            // Add to changed list and remove from maps
            if let Some(extracted_geometry) =
                extracted_geometries.uinode_geometries.remove(&main_entity)
            {
                extracted_geometries
                    .extracted_camera_to_main_uinode_map
                    .get_mut(&extracted_geometry.extracted_camera)
                    .map(|main_entities| main_entities.remove(&main_entity));
            }
        }
    }
}

pub fn extract_uinode_geometry_debug(
    extracted_geometries: bevy_ecs::system::Res<ExtractedUiGeometries>,
) {
    if 0 < extracted_geometries.changed.len() {
        println!("\n");
        println!(
            "camera count: {}",
            extracted_geometries
                .extracted_camera_to_main_uinode_map
                .len()
        );
        println!(
            "node count: {}",
            extracted_geometries.uinode_geometries.len()
        );
        println!("changed count: {}", extracted_geometries.changed.len());
        println!("\n");
    }
}
