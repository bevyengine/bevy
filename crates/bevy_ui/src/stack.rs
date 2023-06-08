//! This module contains the systems that update the stored UI nodes stack

use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;
use bevy_math::Vec2;
use bevy_render::{prelude::Camera, texture::Image};
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};

use crate::{
    prelude::UiCameraConfig, Node, LayoutContext, UiDefaultView, UiLayout, UiLayouts, UiScale,
    UiSurface, UiView, ZIndex,
};

/// List of UI stacks, one for each UI layout
#[derive(Debug, Resource, Default)]
pub struct UiStacks {
    pub stacks: Vec<UiStack>,
}

/// The UI stack for a UI node, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug)]
pub struct UiStack {
    pub view: Entity,
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

/// Maps uinode entities to their camera view
#[derive(Resource, Default, Deref, DerefMut)]
pub struct UiNodeToView(HashMap<Entity, Entity>);

/// Generates the render stack for UI nodes.
///
/// First generate a UI node tree (`StackingContext`) based on z-index.
/// Then flatten that tree into back-to-front ordered `UiStack`.
#[allow(clippy::too_many_arguments)]
pub fn ui_stack_system(
    mut ui_surface: ResMut<UiSurface>,
    mut ui_layouts: ResMut<UiLayouts>,
    mut default_view: ResMut<UiDefaultView>,
    ui_scale: Res<UiScale>,
    image_assets: Res<Assets<Image>>,
    mut removed_cameras: RemovedComponents<Camera>,
    camera_query: Query<(Entity, &Camera, Option<&UiCameraConfig>, Option<&UiScale>)>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    mut ui_stacks: ResMut<UiStacks>,
    root_node_query: Query<(Entity, Option<&UiView>), (With<Node>, Without<Parent>)>,
    zindex_query: Query<&ZIndex, With<Node>>,
    children_query: Query<&Children>,
    mut uinode_map: ResMut<UiNodeToView>,
) {
    ui_stacks.stacks.clear();
    uinode_map.clear();

    // Remove the associated layout root for removed camera entities, if one exists
    for camera_entity in removed_cameras.iter() {
        if let Some(layout_root) = ui_layouts.remove(&camera_entity) {
            let _ = ui_surface.taffy.remove(layout_root.taffy_root);
        }
    }

    let primary_window = primary_window_query.get_single().ok();

    for (view_entity, camera, ui_camera_config, maybe_ui_scale) in camera_query.iter() {
        let local_ui_scale = maybe_ui_scale.map_or(ui_scale.scale, |ui_scale| ui_scale.scale);
        let is_ui_camera = ui_camera_config.map(|inner| inner.show_ui).unwrap_or(true);
        if is_ui_camera {
            // If no default camera, set the first `camera_entity` with UI enabled as the default camera
            default_view.entity.get_or_insert(view_entity);
            let Some(new_context) = camera.target.normalize(primary_window).and_then(|render_target| match render_target {
                    bevy_render::camera::NormalizedRenderTarget::Window(window_ref) =>
                        windows.get(window_ref.entity()).map(|window| LayoutContext::new(Vec2::new(window.physical_width() as f32, window.physical_height() as f32),window.scale_factor(),local_ui_scale,)).ok(),
                    bevy_render::camera::NormalizedRenderTarget::Image(image_handle) => {
                        let image_size = image_assets.get(&image_handle)?.size();
                        Some(LayoutContext::new(image_size, 1.0, 1.0))
                    }
                }) else {
                    continue;
                };
            if let Some(layout) = ui_layouts.get_mut(&view_entity) {
                if new_context != layout.context {
                    ui_surface
                        .taffy
                        .set_style(layout.taffy_root, new_context.root_style())
                        .unwrap();

                    layout.scale_factor_changed =
                        layout.context.combined_scale_factor != new_context.combined_scale_factor;
                    layout.context = new_context;
                    layout.needs_full_update = true;
                } else {
                    layout.scale_factor_changed = false;
                    layout.needs_full_update = false;
                }
            } else {
                let taffy_root = ui_surface.taffy.new_leaf(new_context.root_style()).unwrap();
                ui_layouts.insert(view_entity, UiLayout::new(taffy_root, new_context));
            }
        } else {
            // `camera_entity` not a UI camera so delete its layout root, if it has one
            if let Some(layout) = ui_layouts.remove(&view_entity) {
                let _ = ui_surface.taffy.remove(layout.taffy_root);
            }
            // if `camera_entity` is the default camera, set the default camera to `None`
            if default_view.entity == Some(view_entity) {
                default_view.entity = None;
            }
        }
    }
    for layout in ui_layouts.values_mut() {
        layout.root_uinodes.clear();
    }

    for (root_uinode, maybe_camera) in root_node_query.iter() {
        let maybe_layout = maybe_camera
            .map(|camera| camera.entity)
            .or(default_view.entity)
            .and_then(|camera| ui_layouts.get_mut(&camera));
        if let Some(layout) = maybe_layout {
            layout.root_uinodes.push(root_uinode);
        }
    }

    let mut base_index = 0;
    for (&view, layout) in ui_layouts.iter() {
        // Generate `StackingContext` tree
        let mut global_context = StackingContext::default();
        let mut total_entry_count: usize = 0;

        for root_uinode in &layout.root_uinodes {
            insert_context_hierarchy(
                &zindex_query,
                &children_query,
                *root_uinode,
                &mut global_context,
                None,
                &mut total_entry_count,
            );
        }

        // Flatten `StackingContext` into `UiStack`
        let mut uinodes = Vec::with_capacity(total_entry_count);
        fill_stack_recursively(&mut uinodes, &mut global_context);
        for &uinode in &uinodes {
            uinode_map.insert(uinode, view);
        }
        ui_stacks.stacks.push(UiStack {
            view,
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
