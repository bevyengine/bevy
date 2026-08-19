//! Provides custom example input plugin for the camera. See [`DefaultInputPlugin`].

use bevy::app::prelude::*;
use bevy::camera::{prelude::*, RenderTarget};
use bevy::ecs::prelude::*;
use bevy::input::{
    mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy::math::{prelude::*, DVec2, DVec3};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use bevy::picking::pointer::{
    PointerAction, PointerId, PointerInput, PointerInteraction, PointerLocation, PointerMap,
};

use bevy::camera_controller::pan_orbit_camera::prelude::{
    component::PanOrbitCamera, inputs::MotionInputs,
};

/// The type of mutually exclusive camera motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionKind {
    /// The camera is orbiting and zooming.
    OrbitZoom,
    /// The camera is panning and zooming.
    PanZoom,
    /// The camera is only zooming.
    Zoom,
}

impl From<&MotionInputs> for MotionKind {
    fn from(value: &MotionInputs) -> Self {
        match value {
            MotionInputs::OrbitZoom { .. } => MotionKind::OrbitZoom,
            MotionInputs::PanZoom { .. } => MotionKind::PanZoom,
            MotionInputs::Zoom { .. } => MotionKind::Zoom,
        }
    }
}

/// A plugin that provides a example input mapping.
///
/// The input plugin is responsible for starting motions, sending inputs, and ending motions. See
/// [`PanOrbitCamera`] for more details on how to implement this yourself.
pub struct CustomInputPlugin;
impl Plugin for CustomInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PanOrbitCameraInputMessage>()
            .init_resource::<CameraPointerMap>()
            .add_systems(
                PreUpdate,
                (
                    custom_camera_inputs,
                    PanOrbitCameraInputMessage::receive_messages,
                    PanOrbitCameraInputMessage::send_pointer_inputs,
                )
                    .chain()
                    .after(bevy::picking::PickingSystems::Last)
                    .before(PanOrbitCamera::update_camera_positions),
            );
    }
}

/// A custom implementation of an input system
pub fn custom_camera_inputs(
    pointers: Query<(&PointerId, &PointerLocation)>,
    pointer_map: Res<CameraPointerMap>,
    mut controller: MessageWriter<PanOrbitCameraInputMessage>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    cameras: Query<(Entity, &Camera, &RenderTarget, &PanOrbitCamera)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let orbit_start = MouseButton::Right;
    let pan_start = MouseButton::Left;
    let zoom_stop = 0.0;

    if let Some(&camera) = pointer_map.get(&PointerId::Mouse) {
        let camera_query = cameras.get(camera).ok();
        let is_in_zoom_mode = camera_query
            .map(|(.., editor_cam)| editor_cam.current_motion.is_zooming_only())
            .unwrap_or_default();
        let zoom_amount_abs = camera_query
            .and_then(|(.., editor_cam)| {
                editor_cam
                    .current_motion
                    .inputs()
                    .map(|inputs| inputs.zoom_velocity_abs(editor_cam.smoothing.zoom.mul_f32(2.0)))
            })
            .unwrap_or(0.0);
        let should_zoom_end = is_in_zoom_mode && zoom_amount_abs <= zoom_stop;

        if mouse_input.any_just_released([orbit_start, pan_start]) || should_zoom_end {
            controller.write(PanOrbitCameraInputMessage::End { camera });
        }
    }

    for (&pointer, pointer_location) in pointers
        .iter()
        .filter_map(|(id, loc)| loc.location().map(|loc| (id, loc)))
    {
        match pointer {
            PointerId::Mouse => {
                let Some((camera, ..)) = cameras.iter().find(|(_, camera, render_target, _)| {
                    pointer_location.is_in_viewport(camera, render_target, &primary_window)
                }) else {
                    continue; // Pointer must be in viewport to start a motion.
                };

                if mouse_input.just_pressed(orbit_start) {
                    controller.write(PanOrbitCameraInputMessage::Start {
                        kind: MotionKind::OrbitZoom,
                        camera,
                        pointer,
                    });
                } else if mouse_input.just_pressed(pan_start) {
                    controller.write(PanOrbitCameraInputMessage::Start {
                        kind: MotionKind::PanZoom,
                        camera,
                        pointer,
                    });
                } else if mouse_wheel.read().map(|mw| mw.y.abs()).sum::<f32>() > 0.0 {
                    // Note we can't just check if the mouse wheel inputs are empty, we need to
                    // check if the y value abs greater than zero, otherwise we get a bunch of false
                    // positives, which can cause issues with figuring out what the user is trying
                    // to do.
                    controller.write(PanOrbitCameraInputMessage::Start {
                        kind: MotionKind::Zoom,
                        camera,
                        pointer,
                    });
                }
            }
            PointerId::Touch(_) | PointerId::Custom(_) => continue,
        }
    }

    // This must be cleared manually because reading these inputs is conditional - we are not
    // guaranteed to be flushing the events every frame.
    mouse_wheel.clear();
}

/// Maps pointers to the camera they are currently controlling.
///
/// This is needed so we can automatically track pointer movements and update camera movement after
/// a [`PanOrbitCameraInputMessage::Start`] has been received.
#[derive(Debug, Clone, Default, Deref, DerefMut, Resource)]
pub struct CameraPointerMap(HashMap<PointerId, Entity>);

/// Messages used when implementing input systems for the [`PanOrbitCamera`].
#[derive(Debug, Clone, Message)]
pub enum PanOrbitCameraInputMessage {
    /// Send this event to start moving the camera. The anchor and inputs will be computed
    /// automatically until the [`PanOrbitCameraInputMessage::End`] event is received.
    Start {
        /// The kind of camera movement that is being started.
        kind: MotionKind,
        /// The camera to move.
        camera: Entity,
        /// The pointer that will be controlling the camera. The rotation anchor point in the world
        /// will be automatically computed using picking backends.
        pointer: PointerId,
    },
    /// Send this event when a user's input ends, e.g. the button is released.
    End {
        /// The entity of the camera that should end its current input motion.
        camera: Entity,
    },
}

impl PanOrbitCameraInputMessage {
    /// Get the camera entity associated with this event.
    pub fn camera(&self) -> Entity {
        match self {
            PanOrbitCameraInputMessage::Start { camera, .. }
            | PanOrbitCameraInputMessage::End { camera } => *camera,
        }
    }

    /// Receive [`PanOrbitCameraInputMessage`]s, and use these to start and end moves on the [`PanOrbitCamera`].
    pub fn receive_messages(
        mut events: MessageReader<Self>,
        mut controllers: Query<(&mut PanOrbitCamera, &GlobalTransform)>,
        mut camera_map: ResMut<CameraPointerMap>,
        pointer_map: Res<PointerMap>,
        pointer_interactions: Query<&PointerInteraction>,
        pointer_locations: Query<&PointerLocation>,
        cameras: Query<(&Camera, &Projection)>,
        windows: Query<&Window>,
    ) {
        for event in events.read() {
            let Ok((mut controller, cam_transform)) = controllers.get_mut(event.camera()) else {
                continue;
            };

            match event {
                PanOrbitCameraInputMessage::Start { kind, pointer, .. } => {
                    if controller.is_actively_controlled() {
                        continue;
                    }
                    let anchor = pointer_map
                        .get_entity(*pointer)
                        .and_then(|entity| pointer_interactions.get(entity).ok())
                        .and_then(|interaction| interaction.get_nearest_hit())
                        // Since `bevy` 0.17.3:
                        //
                        // If the current hit is on a window, we cannot use the `hit.position` as an anchor
                        // as the `hit.position` is in viewport coordinates.
                        .filter(|(entity, _hit)| !windows.contains(*entity))
                        .and_then(|(_, hit)| hit.position)
                        .map(|world_space_hit| {
                            // Convert the world space hit to view (camera) space
                            cam_transform
                                .to_matrix()
                                .as_dmat4()
                                .inverse()
                                .transform_point3(world_space_hit.into())
                        })
                        .filter(|p| {
                            #[cfg(debug_assertions)]
                            if !p.is_finite() {
                                bevy::log::warn!(
                                    "Non-finite input fed to camera controller: {p:?}"
                                );
                            }
                            p.is_finite()
                        })
                        .or_else(|| {
                            let camera = cameras.get(event.camera()).ok();
                            let pointer_location = pointer_map
                                .get_entity(*pointer)
                                .and_then(|entity| pointer_locations.get(entity).ok())
                                .and_then(|l| l.location());
                            if let Some(((camera, proj), pointer_location)) =
                                camera.zip(pointer_location)
                            {
                                screen_to_view_space(
                                    camera,
                                    proj,
                                    &controller,
                                    pointer_location.position,
                                )
                            } else {
                                None
                            }
                        })
                        .filter(|p| p.is_finite());

                    match kind {
                        MotionKind::OrbitZoom => controller.start_orbit(anchor),
                        MotionKind::PanZoom => controller.start_pan(anchor),
                        MotionKind::Zoom => controller.start_zoom(anchor),
                    }
                    camera_map.insert(*pointer, event.camera());
                }
                PanOrbitCameraInputMessage::End { .. } => {
                    controller.end_move();
                    if let Some(pointer) = camera_map
                        .iter()
                        .find(|(.., camera)| **camera == event.camera())
                        .map(|(&pointer, ..)| pointer)
                    {
                        camera_map.remove(&pointer);
                    }
                }
            }
        }
    }

    /// While a camera motion is active, this system will take care of sending new pointer motion to
    /// the camera controller. The camera controller assumes that pan and orbit movements are tied
    /// to screen space pointer motion.
    ///
    /// This is because some of the pixel-perfect features of the controller require that data be
    /// passed in as screen space deltas, to compute perfect first-order control. This is also
    /// because the plugin uses pointer information to know which camera is being controlled.
    ///
    /// If you want to control the camera with different inputs, you will need to replace this
    /// system with one that tracks other input methods, and sends the required zoom and screenspace
    /// movement information.
    pub fn send_pointer_inputs(
        camera_map: Res<CameraPointerMap>,
        mut camera_controllers: Query<&mut PanOrbitCamera>,
        mut mouse_wheel: MessageReader<MouseWheel>,
        mut moves: MessageReader<PointerInput>,
    ) {
        let moves_list: Vec<_> = moves.read().collect();
        for (pointer, camera) in camera_map.iter() {
            let Ok(mut camera_controller) = camera_controllers.get_mut(*camera) else {
                continue;
            };

            let screenspace_input = moves_list
                .iter()
                .filter(|m| m.pointer_id.eq(pointer))
                .filter_map(|m| match m.action {
                    PointerAction::Move { delta } => Some(delta),
                    _ => None,
                })
                .sum();

            let zoom_amount = match pointer {
                // TODO: add pinch zoom support
                PointerId::Mouse => mouse_wheel
                    .read()
                    .map(|mw| {
                        let scroll_multiplier = match mw.unit {
                            MouseScrollUnit::Line => 150.0,
                            MouseScrollUnit::Pixel => 1.0,
                        };
                        mw.y * scroll_multiplier
                    })
                    .sum::<f32>(),
                _ => 0.0,
            };

            camera_controller.send_screenspace_input(screenspace_input);
            camera_controller.send_zoom_input(zoom_amount);
        }
        // This must be cleared manually because reading these inputs is conditional - we are not
        // guaranteed to be flushing the events every frame.
        mouse_wheel.clear();
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
