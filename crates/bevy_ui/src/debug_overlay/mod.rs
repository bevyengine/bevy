//! A visual representation of UI node sizes.
use bevy_app::{App, Plugin, PostUpdate};
use bevy_core::Name;
use bevy_core_pipeline::clear_color::ClearColorConfig;
use bevy_core_pipeline::core_2d::{Camera2d, Camera2dBundle};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_gizmos::prelude::{GizmoConfig, Gizmos};
use bevy_hierarchy::{Children, Parent};
use bevy_input::prelude::{Input, KeyCode};
use bevy_log::{info, warn};
use bevy_math::{Vec2, Vec3Swizzles};
use bevy_render::{prelude::*, view::RenderLayers};
use bevy_transform::{prelude::GlobalTransform, TransformSystem};
use bevy_utils::default;
use bevy_window::{PrimaryWindow, Window};

use crate::prelude::UiCameraConfig;
use crate::{Display, Node, Style};
use inset::InsetGizmo;

mod inset;

/// The [`Camera::order`] index used by the layout debug camera.
pub const LAYOUT_DEBUG_CAMERA_ORDER: isize = 255;
/// The [`RenderLayers`] used by the debug gizmos and the debug camera.
pub const LAYOUT_DEBUG_LAYERS: RenderLayers = RenderLayers::none().with(16);

const NODE_LIGHTNESS: f32 = 0.7;
const NODE_SATURATION: f32 = 0.8;

fn hue_from_entity(entity: Entity) -> f32 {
    const FRAC_U32MAX_GOLDEN_RATIO: u32 = 2_654_435_769; // (u32::MAX / Φ) rounded up
    const RATIO_360: f32 = 360.0 / u32::MAX as f32;
    entity.index().wrapping_mul(FRAC_U32MAX_GOLDEN_RATIO) as f32 * RATIO_360
}

#[derive(Clone, Copy)]
struct LayoutRect {
    pos: Vec2,
    size: Vec2,
}

impl LayoutRect {
    fn new(trans: &GlobalTransform, node: &Node) -> Self {
        let mut this = Self {
            pos: trans.translation().xy(),
            size: node.size(),
        };
        this.pos -= this.size / 2.;
        this
    }
}

/// The inputs used by the `bevy_ui` debug overlay.
#[derive(Clone)]
pub struct InputMap {
    /// The key used for enabling/disabling the debug overlay, default is [`KeyCode::F9`].
    pub toggle_key: KeyCode,
}
impl Default for InputMap {
    fn default() -> Self {
        InputMap {
            toggle_key: KeyCode::F9,
        }
    }
}

#[derive(Component, Debug, Clone, Default)]
struct DebugOverlayCamera;

/// The debug overlay options.
#[derive(Resource, Clone, Default)]
pub struct Options {
    /// Whether the overlay is enabled.
    pub enabled: bool,
    /// The inputs used by the debug overlay.
    pub input_map: InputMap,
    layout_gizmos_camera: Option<Entity>,
}

fn update_debug_camera(
    mut gizmo_config: ResMut<GizmoConfig>,
    mut options: ResMut<Options>,
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
        gizmo_config.render_layers = RenderLayers::all();
    } else {
        let spawn_cam = || {
            cmds.spawn((
                UiCameraConfig { show_ui: false },
                Camera2dBundle {
                    projection: OrthographicProjection {
                        far: 1000.0,
                        viewport_origin: Vec2::new(0.0, 0.0),
                        ..default()
                    },
                    camera: Camera {
                        order: LAYOUT_DEBUG_CAMERA_ORDER,
                        ..default()
                    },
                    camera_2d: Camera2d {
                        clear_color: ClearColorConfig::None,
                    },
                    ..default()
                },
                LAYOUT_DEBUG_LAYERS,
                DebugOverlayCamera,
                Name::new("Layout Debug Camera"),
            ))
            .id()
        };
        gizmo_config.enabled = true;
        gizmo_config.render_layers = LAYOUT_DEBUG_LAYERS;
        let cam = *options.layout_gizmos_camera.get_or_insert_with(spawn_cam);
        let Ok(mut cam) = debug_cams.get_mut(cam) else {
            return;
        };
        cam.is_active = true;
    }
}

fn toggle_overlay(input: Res<Input<KeyCode>>, mut options: ResMut<Options>) {
    let map = &options.input_map;
    if input.just_pressed(map.toggle_key) {
        options.enabled = !options.enabled;
        let mode = if options.enabled {
            "Enabled"
        } else {
            "Disabled"
        };
        info!("{mode} UI node preview");
    }
}

fn outline_nodes(outline: &OutlineParam, draw: &mut InsetGizmo, this_entity: Entity) {
    let Ok(to_iter) = outline.children.get(this_entity) else {
        return;
    };
    for (entity, trans, node, style, children) in outline.nodes.iter_many(to_iter) {
        if style.is_none() || style.is_some_and(|s| matches!(s.display, Display::None)) {
            continue;
        }
        let rect = LayoutRect::new(trans, node);
        outline_node(entity, rect, draw);
        if children.is_some() {
            outline_nodes(outline, draw, entity);
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
    gizmo_config: Res<'w, GizmoConfig>,
    children: Query<'w, 's, &'static Children>,
    nodes: Query<'w, 's, NodesQuery>,
}

type CameraQuery<'w, 's> = Query<'w, 's, &'static Camera, With<DebugOverlayCamera>>;

fn outline_roots(
    outline: OutlineParam,
    draw: Gizmos,
    cam: CameraQuery,
    roots: Query<(Entity, &GlobalTransform, &Node), Without<Parent>>,
    window: Query<&Window, With<PrimaryWindow>>,
    nonprimary_windows: Query<&Window, Without<PrimaryWindow>>,
    options: Res<Options>,
) {
    if !options.enabled {
        return;
    }
    if !nonprimary_windows.is_empty() {
        warn!(
            "The layout debug view only uses the primary window scale, \
            you might notice gaps between container lines"
        );
    }
    let scale_factor = Window::scale_factor;
    let window_scale = window.get_single().map_or(1., scale_factor) as f32;
    let line_width = outline.gizmo_config.line_width / window_scale;
    let mut draw = InsetGizmo::new(draw, cam, line_width);
    for (entity, trans, node) in &roots {
        let rect = LayoutRect::new(trans, node);
        outline_node(entity, rect, &mut draw);
        outline_nodes(&outline, &mut draw, entity);
    }
}
fn outline_node(entity: Entity, rect: LayoutRect, draw: &mut InsetGizmo) {
    let hue = hue_from_entity(entity);
    let color = Color::hsl(hue, NODE_SATURATION, NODE_LIGHTNESS);

    draw.rect_2d(rect, color);
    draw.set_scope(rect);
}

/// The debug overlay plugin.
///
/// This spawns a new camera with a low order, and draws gizmo.
///
/// Note that while the debug plugin is enabled, gizmos cannot be used by other
/// cameras.
///
/// disabling the plugin will give you back gizmo control.
pub struct DebugUiPlugin;
impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Options>().add_systems(
            PostUpdate,
            (
                toggle_overlay,
                update_debug_camera,
                outline_roots.after(TransformSystem::TransformPropagate),
            )
                .chain(),
        );
    }
    fn finish(&self, _app: &mut App) {
        info!(
            "The bevy_ui debug overlay is active!\n\
            ----------------------------------------------\n\
            \n\
            This will show the outline of UI nodes.\n\
            Press `F9` to switch between debug mods."
        );
    }
}
