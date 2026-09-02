//! Provides a default input plugin for the camera. See [`DefaultInputPlugin`].

use bevy_app::prelude::*;
use bevy_camera::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_input::mouse::MouseScrollUnit;
use bevy_math::{prelude::*, DVec2, DVec3};
use bevy_platform::collections::HashMap;
use bevy_transform::prelude::*;
use bevy_window::WindowCreated;

use bevy_picking::{
    events::{
        HitDataEvent, PointerDrag, PointerDragEnd, PointerDragStart, PointerEvent, PointerScroll,
    },
    pointer::{PointerButton, PointerId},
};

use crate::pan_orbit_camera::prelude::component::PanOrbitCamera;

/// The type of mutually exclusive camera motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum MotionKind {
    /// The camera is orbiting and zooming.
    OrbitZoom,
    /// The camera is panning and zooming.
    PanZoom,
    /// The camera is only zooming.
    Zoom,
}

/// Input settings for [`PanOrbitCamera`]
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct PanOrbitCameraInputs {
    /// Button to start orbiting
    pub orbit_start: PointerButton,
    /// Button to start panning
    pub pan_start: PointerButton,
    /// Minimum zoom speed/momentum at which to stop zooming
    pub zoom_stop_min: f32,
    /// Zoom factor for [`MouseScrollUnit::Line`]
    pub zoom_line_factor: f32,
    /// Zoom factor for [`MouseScrollUnit::Pixel`]
    pub zoom_pixel_factor: f32,
}
impl Default for PanOrbitCameraInputs {
    fn default() -> Self {
        Self {
            orbit_start: PointerButton::Secondary,
            pan_start: PointerButton::Primary,
            zoom_stop_min: 0.0,
            zoom_line_factor: 150.0,
            zoom_pixel_factor: 1.0,
        }
    }
}
impl PanOrbitCameraInputs {
    fn get_zoom_factor(&self, scroll: MouseScrollUnit) -> f32 {
        match scroll {
            MouseScrollUnit::Line => self.zoom_line_factor,
            MouseScrollUnit::Pixel => self.zoom_pixel_factor,
        }
    }
}
/// Maps pointers to the camera they are currently controlling.
///
/// This is needed so we can automatically track pointer movements and update camera movement after
/// a [`PanOrbitCameraInputMessage::Start`] has been received.
#[derive(Debug, Clone, Default, Deref, DerefMut, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct CameraPointerMap(HashMap<PointerId, Entity>);

/// A plugin that provides a default input mapping. Intended to be replaced by users with their own
/// version of this code, if needed.
///
/// The input plugin is responsible for starting motions, sending inputs, and ending motions. See
/// [`PanOrbitCamera`] for more details on how to implement this yourself.
pub struct DefaultInputPlugin;
impl Plugin for DefaultInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraPointerMap>()
            .add_observer(|evt: On<WindowCreated>, mut commands: Commands| {
                dbg!("Adding window observers");
                commands
                    .entity(evt.window)
                    .observe(observe_window_drag_start)
                    .observe(observe_window_drag)
                    .observe(observe_window_drag_end)
                    .observe(observe_window_scroll);
            })
            // "we have required components at home"
            .add_observer(
                |evt: On<Add<PanOrbitCamera>>,
                 query: Query<&PanOrbitCameraInputs>,
                 mut commands: Commands| {
                    if !query.contains(evt.entity) {
                        commands
                            .entity(evt.entity)
                            .insert(PanOrbitCameraInputs::default());
                    }
                },
            )
            .add_systems(Update, zoom_stop);
    }
}

#[derive(SystemParam)]
struct PanOrbitControllers<'w, 's>(
    Query<
        'w,
        's,
        (
            &'static mut PanOrbitCamera,
            &'static PanOrbitCameraInputs,
            &'static Camera,
            &'static GlobalTransform,
            &'static Projection,
        ),
    >,
);
impl<'w, 's> PanOrbitControllers<'w, 's> {
    fn get_with_anchor<T>(
        &mut self,
        evt: &On<T>,
        original: Entity,
    ) -> Option<(
        Mut<'_, PanOrbitCamera>,
        &PanOrbitCameraInputs,
        Option<DVec3>,
    )>
    where
        T: HitDataEvent + PointerEvent,
    {
        if let Ok((controller, inputs, cam, cam_transform, proj)) = self.0.get_mut(evt.hit().camera)
        {
            let anchor = evt
                .hit()
                .position
                .filter(|_| original != evt.event_target()) // skip hit.position for window drag
                .map(|world_space_hit| {
                    // Convert the world space hit to view (camera) space
                    cam_transform
                        .to_matrix()
                        .as_dmat4()
                        .inverse()
                        .transform_point3(world_space_hit.into())
                })
                .or_else(|| screen_to_view_space(cam, proj, &controller, evt.pointer().position));
            Some((controller, inputs, anchor))
        } else {
            None
        }
    }
}

fn observe_window_scroll(evt: On<PointerScroll>, mut controllers: PanOrbitControllers) {
    if let Some((mut controller, inputs, anchor)) =
        controllers.get_with_anchor(&evt, evt.original_event_target())
    {
        if !controller.is_actively_controlled() {
            controller.start_zoom(anchor);
        }
        controller.send_zoom_input(evt.y * inputs.get_zoom_factor(evt.unit));
    }
}

fn zoom_stop(mut controllers: Query<(&mut PanOrbitCamera, &PanOrbitCameraInputs)>) {
    for (mut controller, inputs) in &mut controllers {
        if controller.current_motion.is_zooming_only()
            && controller
                .current_motion
                .inputs()
                .map(|inputs| inputs.zoom_velocity_abs(controller.smoothing.zoom.mul_f32(2.0)))
                .unwrap_or(0.0)
                <= inputs.zoom_stop_min as f64
        {
            controller.end_move();
        }
    }
}

fn observe_window_drag_start(
    evt: On<PointerDragStart>,
    mut controllers: PanOrbitControllers,
    mut pointer_cameras: ResMut<CameraPointerMap>,
) {
    if let Some((mut controller, inputs, anchor)) =
        controllers.get_with_anchor(&evt, evt.original_event_target())
    {
        if controller.is_actively_controlled() {
            return;
        }

        // dbg!(&anchor);
        if evt.button == inputs.orbit_start {
            controller.start_orbit(anchor);
        } else if evt.button == inputs.pan_start {
            controller.start_pan(anchor);
        } else {
            return;
        }
        pointer_cameras.insert(evt.pointer.id, evt.hit.camera);
    }
}

fn observe_window_drag(
    evt: On<PointerDrag>,
    mut controllers: Query<&mut PanOrbitCamera>,
    pointer_cameras: Res<CameraPointerMap>,
) {
    if let Some(&camera) = pointer_cameras.get(&evt.pointer.id)
        && let Ok(mut controller) = controllers.get_mut(camera)
    {
        controller.send_screenspace_input(evt.delta);
    }
}
fn observe_window_drag_end(
    evt: On<PointerDragEnd>,
    mut controllers: Query<&mut PanOrbitCamera>,
    mut pointer_cameras: ResMut<CameraPointerMap>,
) {
    if let Some(&camera) = pointer_cameras.get(&evt.pointer.id)
        && let Ok(mut controller) = controllers.get_mut(camera)
    {
        controller.end_move();
        dbg!(evt.distance);
        pointer_cameras.remove(&evt.pointer.id);
    }
}

fn screen_to_view_space(
    camera: &Camera,
    proj: &Projection,
    controller: &PanOrbitCamera,
    target_position: Vec2,
) -> Option<DVec3> {
    let mut viewport_position = if let Some(rect) = camera.logical_viewport_rect() {
        target_position.as_dvec2() - rect.min.as_dvec2()
    } else {
        target_position.as_dvec2()
    };
    let target_size = camera.logical_viewport_size()?.as_dvec2();
    // Flip the Y co-ordinate origin from the top to the bottom.
    viewport_position.y = target_size.y - viewport_position.y;
    let ndc = viewport_position * 2. / target_size - DVec2::ONE;
    let ndc_to_view = proj.get_clip_from_view().as_dmat4().inverse();
    let view_near_plane = ndc_to_view.project_point3(ndc.extend(1.));
    match &proj {
        Projection::Perspective(_) | Projection::Custom(_) => {
            // Using EPSILON because an NDC with Z = 0 returns NaNs.
            let view_far_plane = ndc_to_view.project_point3(ndc.extend(f64::EPSILON));
            let direction = (view_far_plane - view_near_plane).normalize();
            Some((direction / direction.z) * controller.last_anchor_depth())
        }
        Projection::Orthographic(_) => Some(DVec3::new(
            view_near_plane.x,
            view_near_plane.y,
            controller.last_anchor_depth(),
        )),
    }
}
