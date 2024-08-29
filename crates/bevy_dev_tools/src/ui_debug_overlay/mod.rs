//! A visual representation of UI node sizes.
use std::any::{Any, TypeId};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_color::Hsla;
use bevy_core::Name;
use bevy_core_pipeline::core_2d::Camera2dBundle;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_gizmos::{config::GizmoConfigStore, prelude::Gizmos, AppGizmoBuilder};
use bevy_hierarchy::{Children, Parent};
use bevy_math::{Vec2, Vec3Swizzles};
use bevy_render::{
    camera::RenderTarget,
    prelude::*,
    view::{RenderLayers, VisibilitySystems},
};
use bevy_transform::{prelude::GlobalTransform, TransformSystem};
use bevy_ui::{DefaultUiCamera, Display, Node, Style, TargetCamera, UiScale};
use bevy_utils::{default, warn_once};
use bevy_window::{PrimaryWindow, Window, WindowRef};

use inset::InsetGizmo;

use self::inset::UiGizmosDebug;

mod inset;

/// The [`Camera::order`] index used by the layout debug camera.
pub const LAYOUT_DEBUG_CAMERA_ORDER: isize = 255;
/// The [`RenderLayers`] used by the debug gizmos and the debug camera.
pub const LAYOUT_DEBUG_LAYERS: RenderLayers = RenderLayers::layer(16);

#[derive(Clone, Copy)]
struct LayoutRect {
    pos: Vec2,
    size: Vec2,
}

impl LayoutRect {
    fn new(trans: &GlobalTransform, node: &Node, scale: f32) -> Self {
        let mut this = Self {
            pos: trans.translation().xy() * scale,
            size: node.size() * scale,
        };
        this.pos -= this.size / 2.;
        this
    }
}

#[derive(Component, Debug, Clone, Default)]
struct DebugOverlayCamera;

/// The debug overlay options.
#[derive(Resource, Clone, Default)]
pub struct UiDebugOptions {
    /// Whether the overlay is enabled.
    pub enabled: bool,
    layout_gizmos_camera: Option<Entity>,
}
impl UiDebugOptions {
    /// This will toggle the enabled field, setting it to false if true and true if false.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

/// The system responsible to change the [`Camera`] config based on changes in [`UiDebugOptions`] and [`GizmoConfig`](bevy_gizmos::prelude::GizmoConfig).
fn update_debug_camera(
    mut gizmo_config: ResMut<GizmoConfigStore>,
    mut options: ResMut<UiDebugOptions>,
    mut cmds: Commands,
    mut debug_cams: Query<&mut Camera, With<DebugOverlayCamera>>,
) {
    if !options.is_changed() && !gizmo_config.is_changed() {
        return;
    }
    if !options.enabled {
        let Some(cam) = options.layout_gizmos_camera else {
            return;
        };
        let Ok(mut cam) = debug_cams.get_mut(cam) else {
            return;
        };
        cam.is_active = false;
        if let Some((config, _)) = gizmo_config.get_config_mut_dyn(&TypeId::of::<UiGizmosDebug>()) {
            config.enabled = false;
        }
    } else {
        let spawn_cam = || {
            cmds.spawn((
                Camera2dBundle {
                    projection: OrthographicProjection {
                        far: 1000.0,
                        viewport_origin: Vec2::new(0.0, 0.0),
                        ..default()
                    },
                    camera: Camera {
                        order: LAYOUT_DEBUG_CAMERA_ORDER,
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    ..default()
                },
                LAYOUT_DEBUG_LAYERS.clone(),
                DebugOverlayCamera,
                Name::new("Layout Debug Camera"),
            ))
            .id()
        };
        if let Some((config, _)) = gizmo_config.get_config_mut_dyn(&TypeId::of::<UiGizmosDebug>()) {
            config.enabled = true;
            config.render_layers = LAYOUT_DEBUG_LAYERS.clone();
        }
        let cam = *options.layout_gizmos_camera.get_or_insert_with(spawn_cam);
        let Ok(mut cam) = debug_cams.get_mut(cam) else {
            return;
        };
        cam.is_active = true;
    }
}

/// The function that goes over every children of given [`Entity`], skipping the not visible ones and drawing the gizmos outlines.
fn outline_nodes(outline: &OutlineParam, draw: &mut InsetGizmo, this_entity: Entity, scale: f32) {
    let Ok(to_iter) = outline.children.get(this_entity) else {
        return;
    };

    for (entity, trans, node, style, children) in outline.nodes.iter_many(to_iter) {
        if style.is_none() || style.is_some_and(|s| matches!(s.display, Display::None)) {
            continue;
        }

        if let Ok(view_visibility) = outline.view_visibility.get(entity) {
            if !view_visibility.get() {
                continue;
            }
        }
        let rect = LayoutRect::new(trans, node, scale);
        outline_node(entity, rect, draw);
        if children.is_some() {
            outline_nodes(outline, draw, entity, scale);
        }
        draw.clear_scope(rect);
    }
}

type NodesQuery = (
    Entity,
    &'static GlobalTransform,
    &'static Node,
    Option<&'static Style>,
    Option<&'static Children>,
);

#[derive(SystemParam)]
struct OutlineParam<'w, 's> {
    gizmo_config: Res<'w, GizmoConfigStore>,
    children: Query<'w, 's, &'static Children>,
    nodes: Query<'w, 's, NodesQuery>,
    view_visibility: Query<'w, 's, &'static ViewVisibility>,
    ui_scale: Res<'w, UiScale>,
}

type CameraQuery<'w, 's> = Query<'w, 's, &'static Camera, With<DebugOverlayCamera>>;

#[derive(SystemParam)]
struct CameraParam<'w, 's> {
    debug_camera: Query<'w, 's, &'static Camera, With<DebugOverlayCamera>>,
    cameras: Query<'w, 's, &'static Camera, Without<DebugOverlayCamera>>,
    primary_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    default_ui_camera: DefaultUiCamera<'w, 's>,
}

/// system responsible for drawing the gizmos lines around all the node roots, iterating recursively through all visible children.
fn outline_roots(
    outline: OutlineParam,
    draw: Gizmos<UiGizmosDebug>,
    cam: CameraParam,
    roots: Query<
        (
            Entity,
            &GlobalTransform,
            &Node,
            Option<&ViewVisibility>,
            Option<&TargetCamera>,
        ),
        Without<Parent>,
    >,
    window: Query<&Window, With<PrimaryWindow>>,
    nonprimary_windows: Query<&Window, Without<PrimaryWindow>>,
    options: Res<UiDebugOptions>,
) {
    if !options.enabled {
        return;
    }
    if !nonprimary_windows.is_empty() {
        warn_once!(
            "The layout debug view only uses the primary window scale, \
            you might notice gaps between container lines"
        );
    }
    let window_scale = window.get_single().map_or(1., Window::scale_factor);
    let scale_factor = outline.ui_scale.0;

    // We let the line be defined by the window scale alone
    let line_width = outline
        .gizmo_config
        .get_config_dyn(&UiGizmosDebug.type_id())
        .map_or(2., |(config, _)| config.line_width)
        / window_scale;
    let mut draw = InsetGizmo::new(draw, cam.debug_camera, line_width);
    for (entity, trans, node, view_visibility, maybe_target_camera) in &roots {
        if let Some(view_visibility) = view_visibility {
            // If the entity isn't visible, we will not draw any lines.
            if !view_visibility.get() {
                continue;
            }
        }
        // We skip ui in other windows that are not the primary one
        if let Some(camera_entity) = maybe_target_camera
            .map(|target| target.0)
            .or(cam.default_ui_camera.get())
        {
            let Ok(camera) = cam.cameras.get(camera_entity) else {
                // The camera wasn't found. Either the Camera don't exist or the Camera is the debug Camera, that we want to skip and warn
                warn_once!("Camera {:?} wasn't found for debug overlay", camera_entity);
                continue;
            };
            match camera.target {
                RenderTarget::Window(window_ref) => {
                    if let WindowRef::Entity(window_entity) = window_ref {
                        if cam.primary_window.get(window_entity).is_err() {
                            // This window isn't the primary, so we skip this root.
                            continue;
                        }
                    }
                }
                // Hard to know the results of this, better skip this target.
                _ => continue,
            }
        }

        let rect = LayoutRect::new(trans, node, scale_factor);
        outline_node(entity, rect, &mut draw);
        outline_nodes(&outline, &mut draw, entity, scale_factor);
    }
}

/// Function responsible for drawing the gizmos lines around the given Entity
fn outline_node(entity: Entity, rect: LayoutRect, draw: &mut InsetGizmo) {
    let color = Hsla::sequential_dispersed(entity.index());

    draw.rect_2d(rect, color.into());
    draw.set_scope(rect);
}

/// The debug overlay plugin.
///
/// This spawns a new camera with a low order, and draws gizmo.
///
/// Note that due to limitation with [`bevy_gizmos`], multiple windows with this feature
/// enabled isn't supported and the lines are only drawn in the [`PrimaryWindow`]
pub struct DebugUiPlugin;
impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiDebugOptions>()
            .init_gizmo_group::<UiGizmosDebug>()
            .add_systems(
                PostUpdate,
                (
                    update_debug_camera,
                    outline_roots
                        .after(TransformSystem::TransformPropagate)
                        // This needs to run before VisibilityPropagate so it can relies on ViewVisibility
                        .before(VisibilitySystems::VisibilityPropagate),
                )
                    .chain(),
            );
    }
}
