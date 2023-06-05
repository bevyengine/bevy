//! This module contains the systems that update the stored UI nodes stack

use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;
use bevy_math::Vec2;
use bevy_render::{prelude::Camera, texture::Image};
use bevy_window::{PrimaryWindow, Window};

use crate::{
    prelude::UiCameraConfig, LayoutContext, Node, UiCameraToRoot, UiDefaultCamera, UiLayoutRoot,
    UiScale, UiSurface, UiTargetCamera, ZIndex,
};

#[derive(Debug, Resource, Default)]
pub struct UiStacks {
    pub stacks: Vec<UiStack>,
}

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug)]
pub struct UiStack {
    pub camera_entity: Entity,
    pub base_index: usize,
    /// List of UI nodes ordered from back-to-front
    pub uinodes: Vec<Entity>,
}

#[derive(Default)]
struct StackingContext {
    pub entries: Vec<StackingContextEntry>,
}

struct StackingContextEntry {
    pub z_index: i32,
    pub entity: Entity,
    pub stack: StackingContext,
}

/// Generates the render stack for UI nodes.
///
/// First generate a UI node tree (`StackingContext`) based on z-index.
/// Then flatten that tree into back-to-front ordered `UiStack`.
pub fn ui_stack_system(
    mut ui_surface: ResMut<UiSurface>,
    mut camera_to_root: ResMut<UiCameraToRoot>,
    mut default_camera: ResMut<UiDefaultCamera>,
    ui_scale: Res<UiScale>,
    image_assets: Res<Assets<Image>>,
    mut removed_cameras: RemovedComponents<Camera>,
    camera_query: Query<(Entity, &Camera, Option<&UiCameraConfig>)>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    mut ui_stacks: ResMut<UiStacks>,
    root_node_query: Query<(Entity, Option<&UiTargetCamera>), (With<Node>, Without<Parent>)>,
    zindex_query: Query<&ZIndex, With<Node>>,
    children_query: Query<&Children>,
) {
    ui_stacks.stacks.clear();

    // Remove the associated layout root for removed camera entities, if one exists
    for camera_entity in removed_cameras.iter() {
        if let Some(layout_root) = camera_to_root.remove(&camera_entity) {
            let _ = ui_surface.taffy.remove(layout_root.taffy_root);
        }
    }

    let primary_window = primary_window_query.get_single().ok();

    for (camera_entity, camera, ui_camera_config) in camera_query.iter() {
        let is_ui_camera = ui_camera_config.map(|inner| inner.show_ui).unwrap_or(true);
        if is_ui_camera {
            // If no default camera, set the first `camera_entity` with UI enabled as the default camera
            default_camera.entity.get_or_insert(camera_entity);
            let Some(layout_context) = camera.target.normalize(primary_window).and_then(|render_target| match render_target {
                    bevy_render::camera::NormalizedRenderTarget::Window(window_ref) =>
                        windows.get(window_ref.entity()).map(|window| LayoutContext::new(Vec2::new(window.physical_width() as f32, window.physical_height() as f32),window.scale_factor(),ui_scale.scale,)).ok(),
                    bevy_render::camera::NormalizedRenderTarget::Image(image_handle) =>
                        image_assets.get(&image_handle).map(|image| LayoutContext::new(image.size(),1.0,ui_scale.scale)),
                }) else {
                    bevy_log::debug!("UI Camera has invalid render target");
                    continue;
                };
            if let Some(layout_root) = camera_to_root.get_mut(&camera_entity) {
                if layout_context != layout_root.context {
                    ui_surface
                        .taffy
                        .set_style(layout_root.taffy_root, layout_context.root_style())
                        .unwrap();
                    layout_root.context = layout_context;
                    layout_root.perform_full_update = true;
                } else {
                    layout_root.perform_full_update = false;
                }
            } else {
                let taffy_root = ui_surface
                    .taffy
                    .new_leaf(layout_context.root_style())
                    .unwrap();
                camera_to_root.insert(camera_entity, UiLayoutRoot::new(taffy_root, layout_context));
            }
        } else {
            // `camera_entity` not a UI camera so delete its layout root, if it has one
            if let Some(layout_root) = camera_to_root.remove(&camera_entity) {
                let _ = ui_surface.taffy.remove(layout_root.taffy_root);
            }
            // if `camera_entity` is the default camera, set the default camera to `None`
            if default_camera.entity == Some(camera_entity) {
                default_camera.entity = None;
            }
        }
    }
    for layout_root in camera_to_root.values_mut() {
        layout_root.root_uinodes.clear();
    }

    for (root_uinode, maybe_camera) in root_node_query.iter() {
        let layout_root = maybe_camera
            .map(|camera| camera.entity)
            .or(default_camera.entity)
            .and_then(|camera| camera_to_root.get_mut(&camera));
        if let Some(layout_root) = layout_root {
            layout_root.root_uinodes.push(root_uinode);
        }
    }

    let mut base_index = 0;
    for (camera_entity, layout_root) in camera_to_root.iter() {
        // Generate `StackingContext` tree
        let mut global_context = StackingContext::default();
        let mut total_entry_count: usize = 0;

        for entity in layout_root.root_uinodes.iter() {
            insert_context_hierarchy(
                &zindex_query,
                &children_query,
                *entity,
                &mut global_context,
                None,
                &mut total_entry_count,
            );
        }

        // Flatten `StackingContext` into `UiStack`
        let mut uinodes = Vec::with_capacity(total_entry_count);
        fill_stack_recursively(&mut uinodes, &mut global_context);
        ui_stacks.stacks.push(UiStack {
            camera_entity: *camera_entity,
            uinodes,
            base_index,
        });
        base_index += total_entry_count;
    }
}

/// Generate z-index based UI node tree
fn insert_context_hierarchy(
    zindex_query: &Query<&ZIndex, With<Node>>,
    children_query: &Query<&Children>,
    entity: Entity,
    global_context: &mut StackingContext,
    parent_context: Option<&mut StackingContext>,
    total_entry_count: &mut usize,
) {
    let mut new_context = StackingContext::default();

    if let Ok(children) = children_query.get(entity) {
        // Reserve space for all children. In practice, some may not get pushed since
        // nodes with `ZIndex::Global` are pushed to the global (root) context.
        new_context.entries.reserve_exact(children.len());

        for entity in children {
            insert_context_hierarchy(
                zindex_query,
                children_query,
                *entity,
                global_context,
                Some(&mut new_context),
                total_entry_count,
            );
        }
    }

    // The node will be added either to global/parent based on its z-index type: global/local.
    let z_index = zindex_query.get(entity).unwrap_or(&ZIndex::Local(0));
    let (entity_context, z_index) = match z_index {
        ZIndex::Local(value) => (parent_context.unwrap_or(global_context), *value),
        ZIndex::Global(value) => (global_context, *value),
    };

    *total_entry_count += 1;
    entity_context.entries.push(StackingContextEntry {
        z_index,
        entity,
        stack: new_context,
    });
}

/// Flatten `StackingContext` (z-index based UI node tree) into back-to-front entities list
fn fill_stack_recursively(result: &mut Vec<Entity>, stack: &mut StackingContext) {
    // Sort entries by ascending z_index, while ensuring that siblings
    // with the same local z_index will keep their ordering. This results
    // in `back-to-front` ordering, low z_index = back; high z_index = front.
    stack.entries.sort_by_key(|e| e.z_index);

    for entry in &mut stack.entries {
        // Parent node renders before/behind childs nodes
        result.push(entry.entity);
        fill_stack_recursively(result, &mut entry.stack);
    }
}
