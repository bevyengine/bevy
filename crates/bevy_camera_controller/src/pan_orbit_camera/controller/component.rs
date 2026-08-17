//! The primary [`Component`] of the controller, [`PanOrbitCamera`].

use std::{
    f32::consts::{FRAC_PI_2, PI},
    time::Duration,
};

use bevy_camera::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::resource::IsResource;
use bevy_log::prelude::*;
use bevy_math::{prelude::*, DAffine3, DMat3, DMat4, DQuat, DVec2, DVec3};
use bevy_platform::time::Instant;
use bevy_time::prelude::*;
use bevy_transform::prelude::*;

use super::transform_adapter::TransformAdapter;
use bevy_window::RequestRedraw;

use super::{
    inputs::MotionInputs,
    momentum::{Momentum, Velocity},
    motion::CurrentMotion,
    projections::{OrthographicSettings, PerspectiveSettings},
    smoothing::{InputQueue, Smoothing},
    zoom::ZoomLimits,
};

/// Tracks all state of a camera's controller, including its inputs, motion, and settings.
///
/// See the documentation on the contained fields and types to learn more about each setting.
///
/// # Moving the Camera
///
/// The [`PanOrbitCameraPlugin`](crate::pan_orbit_camera::DefaultPanOrbitCameraPlugins) will automatically handle sending inputs
/// to the camera controller using [`bevy_picking`] to compute pointer hit locations for mouse,
/// touch, and pen inputs. The picking plugin allows you to specify your own picking backend, or
/// choose from a variety of provided backends. This is important because this camera controller
/// relies on depth information for each pointer, and using the picking plugin means it can do this
/// without forcing you into using a particular hit testing backend, e.g. raycasting, which is used
/// by default.
///
/// To move the camera manually:
///
/// 1. Start a camera motion using one of [`PanOrbitCamera::start_orbit`],  [`PanOrbitCamera::start_pan`],
///    [`PanOrbitCamera::start_zoom`].
/// 2. While the motion should be active, send inputs with [`PanOrbitCamera::send_screenspace_input`] and
///    [`PanOrbitCamera::send_zoom_input`].
/// 3. When the motion should end, call  [`PanOrbitCamera::end_move`].
#[derive(Debug, Clone, Component)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct PanOrbitCamera {
    /// What input motions are currently allowed?
    pub enabled_motion: EnabledMotion,
    /// The type of camera orbit to use.
    pub orbit_constraint: OrbitConstraint,
    /// Set near and far zoom limits, as well as the ability to zoom through objects.
    pub zoom_limits: ZoomLimits,
    /// Input smoothing of camera motion.
    pub smoothing: Smoothing,
    /// Input sensitivity of the camera.
    pub sensitivity: Sensitivity,
    /// Amount of camera momentum after inputs have stopped.
    pub momentum: Momentum,
    /// How long should inputs attempting to start a new motion be ignored, after the last input
    /// ends? This is useful to prevent accidentally killing momentum when, for example, releasing a
    /// two finger right click on a trackpad triggers a scroll input.
    pub input_debounce: Duration,
    /// Settings used when the camera has a perspective [`Projection`].
    pub perspective: PerspectiveSettings,
    /// Settings used when the camera has an orthographic [`Projection`].
    pub orthographic: OrthographicSettings,
    /// Managed by the camera controller, though you may want to change this when spawning or
    /// manually moving the camera.
    ///
    /// If the camera starts moving, but there is nothing under the pointer, the controller will
    /// rotate, pan, and zoom about a point in the direction the camera is facing, at this depth.
    /// This will be overwritten with the latest depth if a hit is found, to ensure the anchor point
    /// doesn't change suddenly if the user moves the pointer away from an object.
    pub last_anchor_depth: f64,
    /// Current camera motion. Managed by the camera controller, but exposed publicly to allow for
    /// overriding motion.
    pub current_motion: CurrentMotion,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            orbit_constraint: Default::default(),
            zoom_limits: Default::default(),
            smoothing: Default::default(),
            sensitivity: Default::default(),
            momentum: Default::default(),
            input_debounce: Duration::from_millis(80),
            perspective: Default::default(),
            orthographic: Default::default(),
            enabled_motion: Default::default(),
            current_motion: Default::default(),
            last_anchor_depth: -2.0,
        }
    }
}

impl PanOrbitCamera {
    /// Create a new editor camera component.
    pub fn new(
        orbit: OrbitConstraint,
        smoothness: Smoothing,
        sensitivity: Sensitivity,
        momentum: Momentum,
        initial_anchor_depth: f64,
    ) -> Self {
        Self {
            orbit_constraint: orbit,
            smoothing: smoothness,
            sensitivity,
            momentum,
            last_anchor_depth: -initial_anchor_depth.abs(), // ensure depth is correct sign
            ..Default::default()
        }
    }

    /// Set the initial anchor depth of the camera controller.
    pub fn with_initial_anchor_depth(self, initial_anchor_depth: f64) -> Self {
        Self {
            last_anchor_depth: -initial_anchor_depth.abs(), // ensure depth is correct sign
            ..self
        }
    }

    /// Gets the [`MotionInputs`], if the camera is being actively moved..
    pub fn motion_inputs(&self) -> Option<&MotionInputs> {
        match &self.current_motion {
            CurrentMotion::Stationary => None,
            CurrentMotion::Momentum { .. } => None,
            CurrentMotion::UserControlled { motion_inputs, .. } => Some(motion_inputs),
        }
    }

    /// Returns the best guess at an anchor point if none is provided.
    ///
    /// Updates the fallback value with the latest hit. Ensures that if the camera starts orbiting
    /// again and the pointer is not hitting anything, the anchor doesn't suddenly change distance.
    /// This is what would happen if we used a fixed value.
    fn maybe_update_anchor(&mut self, anchor: Option<DVec3>) -> DVec3 {
        let validate_anchor =
            |anchor: &DVec3| anchor.length() >= f32::EPSILON as f64 && anchor.is_finite();

        let z_last = -self.last_anchor_depth.abs();
        let fallback = anchor
            .filter(|a| a.is_finite())
            .map(|mut anchor| {
                anchor.z = z_last;
                anchor
            })
            .filter(validate_anchor)
            .unwrap_or(DVec3::new(0.0, 0.0, z_last));

        let anchor = anchor.filter(validate_anchor).unwrap_or(fallback);

        self.last_anchor_depth = anchor.z;
        anchor
    }

    /// Get the position of the anchor in the camera's view space.
    pub fn anchor_view_space(&self) -> Option<DVec3> {
        if let CurrentMotion::UserControlled { anchor, .. } = &self.current_motion {
            Some(*anchor)
        } else {
            None
        }
    }

    /// Get the position of the anchor in world space.
    pub fn anchor_world_space(&self, camera_transform: &GlobalTransform) -> Option<DVec3> {
        self.anchor_view_space().map(|anchor_view_space| {
            camera_transform
                .to_matrix()
                .as_dmat4()
                .transform_point3(anchor_view_space)
        });

        self.anchor_view_space().map(|anchor_view_space| {
            let (_, r, t) = camera_transform.to_scale_rotation_translation();
            r.as_dquat() * anchor_view_space + t.as_dvec3()
        })
    }

    /// Should the camera controller prevent new motions from starting because the user is actively
    /// operating the camera?
    ///
    /// This does not consider zooming as "actively controlled". This is needed because scroll input
    /// devices often have their own momentum and can continue to provide values even when the user
    /// is not actively providing inputs. Like a scroll wheel that keeps spinning or a trackpad
    /// with smooth scrolling. Without this, the controller will feel unresponsive, as a user will
    /// be unable to initiate a new motion even though they are not technically providing an input.
    pub fn is_actively_controlled(&self) -> bool {
        !self.current_motion.is_zooming_only()
            && (self.current_motion.is_user_controlled()
                || self
                    .current_motion
                    .momentum_duration()
                    .map(|duration| duration < self.input_debounce)
                    .unwrap_or(false))
    }

    /// Call this to start an orbiting motion with the optionally supplied anchor position in view
    /// space. See [`PanOrbitCamera`] for usage.
    pub fn start_orbit(&mut self, anchor: Option<DVec3>) {
        if !self.enabled_motion.orbit {
            return;
        }
        self.current_motion = CurrentMotion::UserControlled {
            anchor: self.maybe_update_anchor(anchor),
            motion_inputs: MotionInputs::OrbitZoom {
                screenspace_inputs: InputQueue::default(),
                zoom_inputs: InputQueue::default(),
            },
        }
    }

    /// Call this to start a panning motion with the optionally supplied anchor position in view
    /// space. See [`PanOrbitCamera`] for usage.
    pub fn start_pan(&mut self, anchor: Option<DVec3>) {
        if !self.enabled_motion.pan {
            return;
        }
        self.current_motion = CurrentMotion::UserControlled {
            anchor: self.maybe_update_anchor(anchor),
            motion_inputs: MotionInputs::PanZoom {
                screenspace_inputs: InputQueue::default(),
                zoom_inputs: InputQueue::default(),
            },
        }
    }

    /// Call this to start a zooming motion with the optionally supplied anchor position in view
    /// space. See [`PanOrbitCamera`] for usage.
    pub fn start_zoom(&mut self, anchor: Option<DVec3>) {
        if !self.enabled_motion.zoom {
            return;
        }
        let anchor = self.maybe_update_anchor(anchor);

        // Inherit current camera velocity
        let zoom_inputs = match self.current_motion {
            CurrentMotion::Stationary | CurrentMotion::Momentum { .. } => InputQueue::default(),
            CurrentMotion::UserControlled {
                ref mut motion_inputs,
                ..
            } => InputQueue(motion_inputs.zoom_inputs_mut().0.drain(..).collect()),
        };
        self.current_motion = CurrentMotion::UserControlled {
            anchor,
            motion_inputs: MotionInputs::Zoom { zoom_inputs },
        }
    }

    /// Send screen space camera inputs. This will be interpreted as panning or orbiting depending
    /// on the current motion. See [`PanOrbitCamera`] for usage.
    pub fn send_screenspace_input(&mut self, screenspace_input: Vec2) {
        if let CurrentMotion::UserControlled {
            ref mut motion_inputs,
            ..
        } = self.current_motion
        {
            match motion_inputs {
                MotionInputs::OrbitZoom {
                    screenspace_inputs: movement,
                    ..
                } => movement.process_input(screenspace_input, self.smoothing.orbit),
                MotionInputs::PanZoom {
                    screenspace_inputs: movement,
                    ..
                } => movement.process_input(screenspace_input, self.smoothing.pan),
                MotionInputs::Zoom { .. } => (), // When in zoom-only, we ignore pan and zoom
            }
        }
    }

    /// Send zoom inputs. See [`PanOrbitCamera`] for usage.
    pub fn send_zoom_input(&mut self, zoom_amount: f32) {
        if let CurrentMotion::UserControlled { motion_inputs, .. } = &mut self.current_motion {
            motion_inputs
                .zoom_inputs_mut()
                .process_input(zoom_amount, self.smoothing.zoom)
        }
    }

    /// End the current camera motion, allowing other motions on this camera to begin. See
    /// [`PanOrbitCamera`] for usage.
    pub fn end_move(&mut self) {
        let velocity = match self.current_motion {
            CurrentMotion::Stationary => return,
            CurrentMotion::Momentum { .. } => return,
            CurrentMotion::UserControlled {
                anchor,
                ref motion_inputs,
                ..
            } => match motion_inputs {
                MotionInputs::OrbitZoom { .. } => Velocity::Orbit {
                    anchor,
                    velocity: motion_inputs.orbit_momentum(self.momentum.init_orbit),
                },
                MotionInputs::PanZoom { .. } => Velocity::Pan {
                    anchor,
                    velocity: motion_inputs.pan_momentum(self.momentum.init_pan),
                },
                MotionInputs::Zoom { .. } => Velocity::None,
            },
        };
        let momentum_start = Instant::now();
        self.current_motion = CurrentMotion::Momentum {
            velocity,
            momentum_start,
        };
    }

    /// Called once every frame to compute motions and update the transforms of all [`PanOrbitCamera`]s
    pub fn update_camera_positions(
        mut camera_set: ParamSet<(
            Query<EntityRef, (With<PanOrbitCamera>, Without<IsResource>)>,
            Query<(&mut PanOrbitCamera, &Camera, Mut<Projection>), Without<IsResource>>,
            Query<EntityMut, (With<PanOrbitCamera>, Without<IsResource>)>,
        )>,
        transform_adapter: Res<TransformAdapter>,
        mut event: MessageWriter<RequestRedraw>,
        time: Res<Time>,
    ) {
        camera_set
            .p0()
            .iter()
            .filter_map(|entity_ref| {
                transform_adapter
                    .read(&entity_ref)
                    .map(|transform| (entity_ref.id(), transform))
            })
            .collect::<Vec<_>>()
            .iter()
            .filter_map(|(entity, (original_translation, original_rotation))| {
                camera_set
                    .p1()
                    .get_mut(*entity)
                    .ok()
                    .and_then(|(mut camera_controller, camera, projection)| {
                        let dt = time.delta();
                        camera_controller.update_transform_and_projection(
                            camera,
                            original_translation,
                            original_rotation,
                            projection,
                            &mut event,
                            dt,
                        )
                    })
                    .map(|transform| (*entity, transform))
            })
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(entity, (delta_translation, delta_rotation))| {
                if let Ok(mut entity_mut) = camera_set.p2().get_mut(*entity) {
                    transform_adapter.apply_delta(
                        &mut entity_mut,
                        *delta_translation,
                        *delta_rotation,
                    );
                }
            });
    }

    /// Update this [`PanOrbitCamera`]'s transform and projection.
    pub fn update_transform_and_projection(
        &mut self,
        camera: &Camera,
        original_translation: &DVec3,
        original_rotation: &DQuat,
        mut projection: Mut<Projection>,
        redraw: &mut MessageWriter<RequestRedraw>,
        delta_time: Duration,
    ) -> Option<(DVec3, DQuat)> {
        let mut new_translation = *original_translation;
        let mut new_rotation = *original_rotation;
        let (anchor, orbit, pan, zoom) = match &mut self.current_motion {
            CurrentMotion::Stationary => return None,
            CurrentMotion::Momentum { velocity, .. } => {
                velocity.decay(self.momentum, delta_time);
                match velocity {
                    Velocity::None => {
                        self.current_motion = CurrentMotion::Stationary;
                        return None;
                    }
                    Velocity::Orbit { anchor, velocity } => (anchor, *velocity, DVec2::ZERO, 0.0),
                    Velocity::Pan { anchor, velocity } => (anchor, DVec2::ZERO, *velocity, 0.0),
                }
            }
            CurrentMotion::UserControlled {
                anchor,
                motion_inputs,
            } => (
                anchor,
                motion_inputs.smooth_orbit_velocity() * self.sensitivity.orbit.as_dvec2(),
                motion_inputs.smooth_pan_velocity(),
                motion_inputs.smooth_zoom_velocity() * self.sensitivity.zoom as f64,
            ),
        };

        // If there is no motion, we will have already early-exited.
        redraw.write(RequestRedraw);

        let screen_to_view_space_at_depth =
            |perspective: &PerspectiveProjection, depth: f64| -> Option<DVec2> {
                let target_size = camera.logical_viewport_size()?.as_dvec2();
                // This is a strange-looking, but key part of the otherwise normal-looking
                // screen-to-view transformation. What we are trying to do here is answer "if we
                // move by one pixel in x and y, how much distance do we cover in the world at the
                // specified depth?" Because the viewport position's origin is in the corner, we
                // need to halve the target size and subtract one pixel. This gets us a viewport
                // position one pixel diagonal offset from the center of the screen.
                let mut viewport_position = target_size / 2.0 - 1.0;
                // Flip the y-coordinate origin from the top to the bottom.
                viewport_position.y = target_size.y - viewport_position.y;
                let ndc = viewport_position * 2. / target_size - DVec2::ONE;

                let ndc_to_view = DMat4::perspective_infinite_reverse_rh(
                    perspective.fov as f64,
                    perspective.aspect_ratio as f64,
                    perspective.near as f64,
                )
                .inverse(); // f64 version replaced .get_projection_matrix().as_dmat4().inverse();

                let view_near_plane = ndc_to_view.project_point3(ndc.extend(1.));
                // Using EPSILON because an NDC with Z = 0 returns NaNs.
                let view_far_plane = ndc_to_view.project_point3(ndc.extend(f64::EPSILON));
                let direction = view_far_plane - view_near_plane;
                let depth_normalized_direction = direction / direction.z;
                let view_pos3 = depth_normalized_direction * depth;
                let view_pos = view_pos3.truncate();
                if !view_pos.is_finite() || view_pos3.z != depth {
                    #[cfg(debug_assertions)]
                    error!("Invalid view position {view_pos:?} from depth {depth}");
                    return None;
                }
                Some(view_pos)
            };

        let view_offset = match projection.as_ref() {
            Projection::Perspective(perspective) => {
                let Some(offset) = screen_to_view_space_at_depth(perspective, anchor.z) else {
                    error!("Malformed camera");
                    return None;
                };
                offset
            }
            Projection::Orthographic(ortho) => DVec2::new(-ortho.scale as f64, ortho.scale as f64),
            Projection::Custom(_) => {
                error_once!("Custom projections are not supported.");
                return None;
            }
        };

        let pan_translation_view_space = (pan * view_offset).extend(0.0);

        let size_at_anchor =
            super::zoom::length_per_pixel_at_view_space_pos(camera, *anchor).unwrap_or(0.0);

        // I'm not sure why I created this mapping - maybe it was to prevent zooming through
        // surfaces if the user really whipped the mouse:
        //
        // let zoom_unscaled = (zoom.abs() / 60.0)
        //     .powf(1.3); // Varies from 0 to 1 over x = [0..inf]
        // let zoom_input = (1.0 - 1.0 / (zoom_unscaled + 1.0)) * zoom.signum();
        //
        // It is roughly equivalent to just using
        // let zoom_input = zoom * 0.01;
        //
        // ...so I've opted to just factor this constant out of the other scaling constants below.
        //
        // I recall spending a lot of time on this mapping function, but for the life of me can't
        // remember why. Leaving this comment behind for a few releases, delete me if nothing
        // breaks.

        // The zoom input, bounded to prevent zooming past the limits.
        let zoom_bounded = if size_at_anchor <= self.zoom_limits.min_size_per_pixel {
            zoom.min(0.0) // Prevent zooming in further
        } else if size_at_anchor >= self.zoom_limits.max_size_per_pixel {
            zoom.max(0.0) // Prevent zooming out further
        } else {
            zoom
        };

        let zoom_translation_view_space = match &mut *projection {
            Projection::Perspective(perspective) => {
                let zoom_amount = if self.zoom_limits.zoom_through_objects {
                    // Clamp the zoom speed at the limits
                    zoom * size_at_anchor.clamp(
                        self.zoom_limits.min_size_per_pixel,
                        self.zoom_limits.max_size_per_pixel,
                    )
                } else {
                    // If we cannot zoom through objects, use the bounded input
                    zoom_bounded * size_at_anchor
                };
                // Scale this with the perspective FOV, so the zoom speed feels the same regardless.
                anchor.normalize() * zoom_amount / perspective.fov as f64
            }
            Projection::Orthographic(ortho) => {
                // Constants are hand-tuned to feel equivalent between perspective and ortho. Might
                // be a better way to do this correctly if it matters.
                ortho.scale *= 1.0 - zoom_bounded as f32 * 0.0015;
                // We don't move the camera in z, as this is managed by another ortho system.
                anchor.normalize()
                    * zoom_bounded
                    * anchor.z.abs()
                    * 0.0015
                    * DVec3::new(1.0, 1.0, 0.0)
            }
            Projection::Custom(_) => {
                error_once!("Custom projections are not supported.");
                return None;
            }
        };

        // If we can zoom through objects, then scoot the anchor point forward when we hit the
        // limit. This prevents the anchor from getting closer to the camera than the minimum
        // distance, or worse, zooming past the anchor.
        if self.zoom_limits.zoom_through_objects
            && size_at_anchor < self.zoom_limits.min_size_per_pixel
            && matches!(*projection, Projection::Perspective(_))
            && zoom > 0.0
        {
            *anchor += zoom_translation_view_space;
        }

        new_translation +=
            new_rotation * (pan_translation_view_space + zoom_translation_view_space);

        *anchor -= pan_translation_view_space + zoom_translation_view_space;

        let orbit = orbit * DVec2::new(-1.0, 1.0);
        let anchor_world = DMat4::from_rotation_translation(new_rotation, new_translation)
            .transform_point3(*anchor);
        let orbit_dir = orbit.normalize().extend(0.0);
        let orbit_axis_world = new_rotation
            .mul_vec3(orbit_dir.cross(DVec3::NEG_Z).normalize())
            .normalize();

        let orbit_multiplier = 0.005;
        if orbit.is_finite() && orbit.length() != 0.0 {
            match self.orbit_constraint {
                OrbitConstraint::Fixed { up, can_pass_tdc } => {
                    let epsilon = 1e-3;
                    let motion_threshold = 1e-5;

                    let angle_to_bdc = cam_forward(new_rotation).angle_between(up);
                    let angle_to_tdc = cam_forward(new_rotation).angle_between(-up);
                    let pitch_angle = {
                        let desired_rotation = orbit.y * orbit_multiplier;
                        if can_pass_tdc {
                            desired_rotation
                        } else if desired_rotation >= 0.0 {
                            desired_rotation.min(angle_to_tdc - (epsilon as f64).min(angle_to_tdc))
                        } else {
                            desired_rotation.max(-angle_to_bdc + (epsilon as f64).min(angle_to_bdc))
                        }
                    };
                    let pitch = if pitch_angle.abs() <= motion_threshold {
                        DQuat::IDENTITY
                    } else {
                        DQuat::from_axis_angle(cam_left(new_rotation), pitch_angle)
                    };

                    let yaw_angle = orbit.x * orbit_multiplier;
                    let yaw = if yaw_angle.abs() <= motion_threshold {
                        DQuat::IDENTITY
                    } else {
                        DQuat::from_axis_angle(up, yaw_angle)
                    };

                    match [pitch == DQuat::IDENTITY, yaw == DQuat::IDENTITY] {
                        [true, true] => (),
                        [true, false] => rotate_around(
                            (&mut new_translation, &mut new_rotation),
                            anchor_world,
                            yaw,
                        ),
                        [false, true] => rotate_around(
                            (&mut new_translation, &mut new_rotation),
                            anchor_world,
                            pitch,
                        ),
                        [false, false] => rotate_around(
                            (&mut new_translation, &mut new_rotation),
                            anchor_world,
                            yaw * pitch,
                        ),
                    };

                    let how_upright = cam_up(new_rotation).angle_between(up).abs() as f32;
                    // Orient the camera so up always points up (roll).
                    let forward = cam_forward(new_rotation);
                    if how_upright > epsilon && how_upright < FRAC_PI_2 - epsilon {
                        new_rotation = look_to(forward, up);
                    } else if how_upright > FRAC_PI_2 + epsilon && how_upright < PI - epsilon {
                        new_rotation = look_to(forward, -up);
                    }
                }
                OrbitConstraint::Free => {
                    let rotation =
                        DQuat::from_axis_angle(orbit_axis_world, orbit.length() * orbit_multiplier);
                    rotate_around(
                        (&mut new_translation, &mut new_rotation),
                        anchor_world,
                        rotation,
                    );
                }
            }
        }

        self.last_anchor_depth = anchor.z;
        let (_, delta_rotation, delta_translation) = {
            let original =
                DAffine3::from_rotation_translation(*original_rotation, *original_translation);
            let new = DAffine3::from_rotation_translation(new_rotation, new_translation);
            (original.inverse() * new).to_scale_rotation_translation()
        };
        Some((delta_translation, delta_rotation))
    }

    /// Compute the world space size of a pixel at the anchor.
    ///
    /// This is a robust alternative to using the distance of the camera from the anchor point.
    /// Camera distance is not directly related to how large something is on screen - that depends
    /// on the camera projection.
    ///
    /// This function correctly accounts for camera projection and is particularly useful when
    /// doing zoom and scale calculations.
    pub fn length_per_pixel_at_anchor(&self, camera: &Camera) -> Option<f64> {
        let anchor_view = self.anchor_view_space()?;
        super::zoom::length_per_pixel_at_view_space_pos(camera, anchor_view)
    }

    /// The last known anchor depth. This value will always be negative.
    pub fn last_anchor_depth(&self) -> f64 {
        -self.last_anchor_depth.abs()
    }
}

/// A 64-bit version of Transform::rotate_around
pub fn rotate_around(transform: (&mut DVec3, &mut DQuat), point: DVec3, rotation: DQuat) {
    *transform.0 = point + rotation * (*transform.0 - point);
    *transform.1 = (rotation * *transform.1).normalize();
}

/// A 64-bit version of Transform::look_to. Returns the rotation quaternion for the given
/// facing direction and up vector.
pub fn look_to(direction: DVec3, up: DVec3) -> DQuat {
    let back = -direction;
    let right = up
        .cross(back)
        .try_normalize()
        .unwrap_or_else(|| up.any_orthogonal_vector());
    let up = back.cross(right);
    DQuat::from_mat3(&DMat3::from_cols(right, up, back))
}

/// Helper method for getting a local forward vector
pub fn cam_forward(cam_rotation: DQuat) -> DVec3 {
    cam_rotation * DVec3::NEG_Z
}
/// Helper method for getting a local left vector
pub fn cam_left(cam_rotation: DQuat) -> DVec3 {
    cam_rotation * DVec3::NEG_X
}
/// Helper method for getting a local up vector
pub fn cam_up(cam_rotation: DQuat) -> DVec3 {
    cam_rotation * DVec3::Y
}

/// Settings that define how camera orbit behaves.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum OrbitConstraint {
    /// The camera's up direction is fixed.
    Fixed {
        /// The camera's up direction must always be parallel with this unit vector.
        up: DVec3,
        /// Should the camera be allowed to pass over top dead center (TDC), making the camera
        /// upside down compared to the up direction?
        can_pass_tdc: bool,
    },
    /// The camera's up direction is free.
    Free,
}

impl Default for OrbitConstraint {
    fn default() -> Self {
        Self::Fixed {
            up: DVec3::Y,
            can_pass_tdc: false,
        }
    }
}

/// The sensitivity of the camera controller to inputs.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct Sensitivity {
    /// X/Y sensitivity of orbit inputs, multiplied.
    pub orbit: Vec2,
    /// Sensitivity of zoom inputs, multiplied.
    pub zoom: f32,
}

impl Default for Sensitivity {
    fn default() -> Self {
        Self {
            orbit: Vec2::splat(1.0),
            zoom: 1.0,
        }
    }
}

/// Controls what kinds of motions are allowed to initiate. Does not affect momentum.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct EnabledMotion {
    /// Should pan be enabled?
    pub pan: bool,
    /// Should orbit be enabled?
    pub orbit: bool,
    /// Should zoom be enabled?
    pub zoom: bool,
}

impl Default for EnabledMotion {
    fn default() -> Self {
        Self {
            pan: true,
            orbit: true,
            zoom: true,
        }
    }
}
