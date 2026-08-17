//! A `bevy_editor_cam` extension that adds the ability to smoothly rotate the camera about its
//! anchor point until it is looking in the specified direction.

use std::{f64::consts::PI, time::Duration};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::resource::IsResource;
use bevy_math::{prelude::*, DAffine3, DQuat, DVec3};
use bevy_platform::{collections::HashMap, time::Instant};
use bevy_window::RequestRedraw;

use crate::pan_orbit_camera::prelude::*;

/// See the [module](self) docs.
pub struct LookToPlugin;

impl Plugin for LookToPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookTo>()
            .add_message::<LookToTrigger>()
            .add_systems(
                PreUpdate,
                LookTo::update.before(PanOrbitCamera::update_camera_positions),
            )
            .add_systems(PostUpdate, LookToTrigger::receive); // In PostUpdate so we don't miss users sending this in Update. LookTo::update will catch the changes next frame.
    }
}

/// Send this event to rotate the camera about its anchor until it is looking in the given direction
/// with the given up direction. Animation speed is configured with the [`LookTo`] resource.
#[derive(Debug, Message)]
pub struct LookToTrigger {
    /// The new direction to face.
    pub target_facing_direction: DVec3,
    /// The camera's "up" direction when finished moving.
    pub target_up_direction: DVec3,
    /// The camera to update.
    pub camera: Entity,
}

impl LookToTrigger {
    /// Constructs a [`LookToTrigger`] with the up direction automatically selected.
    ///
    /// If the camera is set to [`OrbitConstraint::Fixed`], the fixed up direction will be used, as
    /// long as it is not parallel to the facing direction. If set to [`OrbitConstraint::Free`] or
    /// the facing direction is parallel to the fixed up direction, the up direction will be
    /// automatically selected by choosing the axis that results in the least amount of rotation.
    pub fn auto_snap_up_direction(
        facing: DVec3,
        cam_entity: Entity,
        cam_rotation: &DQuat,
        cam_editor: &PanOrbitCamera,
    ) -> Self {
        const EPSILON: f64 = 0.01;
        let constraint = match cam_editor.orbit_constraint {
            OrbitConstraint::Fixed { up, .. } => Some(up),
            OrbitConstraint::Free => None,
        }
        .filter(|up| {
            let angle = facing.angle_between(*up).abs();
            angle > EPSILON && angle < PI - EPSILON
        });

        let up = constraint.unwrap_or_else(|| {
            let current = cam_rotation;
            let options = [
                DVec3::X,
                DVec3::NEG_X,
                DVec3::Y,
                DVec3::NEG_Y,
                DVec3::Z,
                DVec3::NEG_Z,
            ];
            *options
                .iter()
                .map(|d| (d, look_to(facing, *d)))
                .map(|(d, rot)| (d, rot.angle_between(*current).abs()))
                .reduce(|acc, this| if this.1 < acc.1 { this } else { acc })
                .map(|nearest| nearest.0)
                .unwrap_or(&DVec3::Y)
        });

        LookToTrigger {
            target_facing_direction: facing,
            target_up_direction: up.normalize(),
            camera: cam_entity,
        }
    }
}

impl LookToTrigger {
    fn receive(
        mut events: MessageReader<Self>,
        mut state: ResMut<LookTo>,
        mut camera_set: ParamSet<(
            Query<&mut PanOrbitCamera, Without<IsResource>>,
            Query<EntityRef, (With<PanOrbitCamera>, Without<IsResource>)>,
        )>,
        mut redraw: MessageWriter<RequestRedraw>,
        transform_adapter: Res<TransformAdapter>,
    ) {
        for event in events.read() {
            let camera_refs = camera_set.p1();
            let Ok(camera_ref) = camera_refs.get(event.camera) else {
                continue;
            };
            let Some((_, camera_rotation)) = transform_adapter.read(&camera_ref) else {
                continue;
            };
            let mut cameras = camera_set.p0();
            let Ok(mut controller) = cameras.get_mut(event.camera) else {
                continue;
            };
            redraw.write(RequestRedraw);
            let camera_forward = camera_rotation * DVec3::NEG_Z;
            let camera_up = camera_rotation * DVec3::Y;
            state
                .map
                .entry(event.camera)
                .and_modify(|e| {
                    e.start = Instant::now();
                    e.initial_facing_direction = camera_forward;
                    e.initial_up_direction = camera_up;
                    e.target_facing_direction = event.target_facing_direction;
                    e.target_up_direction = event.target_up_direction;
                    e.complete = false;
                })
                .or_insert(LookToEntry {
                    start: Instant::now(),
                    initial_facing_direction: camera_forward,
                    initial_up_direction: camera_up,
                    target_facing_direction: event.target_facing_direction,
                    target_up_direction: event.target_up_direction,
                    complete: false,
                });

            controller.end_move();
            controller.current_motion = motion::CurrentMotion::Stationary;
        }
    }
}

struct LookToEntry {
    start: Instant,
    initial_facing_direction: DVec3,
    initial_up_direction: DVec3,
    target_facing_direction: DVec3,
    target_up_direction: DVec3,
    complete: bool,
}

/// Stores settings and state for the dolly zoom plugin.
#[derive(Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct LookTo {
    /// The duration of the "look to" transition animation.
    pub animation_duration: Duration,
    /// The cubic curve used to animate the camera during a "look to".
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub animation_curve: CubicSegment<Vec2>,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    map: HashMap<Entity, LookToEntry>,
}

impl Default for LookTo {
    fn default() -> Self {
        Self {
            animation_duration: Duration::from_millis(400),
            animation_curve: CubicSegment::new_bezier_easing((0.25, 0.0), (0.25, 1.0)),
            map: Default::default(),
        }
    }
}

impl LookTo {
    fn update(
        mut state: ResMut<Self>,
        mut camera_set: ParamSet<(
            Query<&mut PanOrbitCamera, Without<IsResource>>,
            Query<EntityRef, (With<PanOrbitCamera>, Without<IsResource>)>,
            Query<EntityMut, (With<PanOrbitCamera>, Without<IsResource>)>,
        )>,
        mut redraw: MessageWriter<RequestRedraw>,
        transform_adapter: Res<TransformAdapter>,
    ) {
        let animation_duration = state.animation_duration;
        let animation_curve = state.animation_curve;
        for (
            camera,
            LookToEntry {
                start,
                initial_facing_direction,
                initial_up_direction,
                target_facing_direction,
                target_up_direction,
                complete,
            },
        ) in state.map.iter_mut()
        {
            let camera_refs = camera_set.p1();
            let Ok(camera_ref) = camera_refs.get(*camera) else {
                continue;
            };
            let Some((mut camera_translation, mut camera_rotation)) =
                transform_adapter.read(&camera_ref)
            else {
                continue;
            };
            let mut cameras = camera_set.p0();
            let Ok(controller) = cameras.get_mut(*camera) else {
                *complete = true;
                continue;
            };
            let progress_t =
                (start.elapsed().as_secs_f32() / animation_duration.as_secs_f32()).clamp(0.0, 1.0);
            let progress = animation_curve.ease(progress_t);

            let anchor_view_space = controller.anchor_view_space().unwrap_or(DVec3::new(
                0.0,
                0.0,
                controller.last_anchor_depth(),
            ));

            let anchor_world = {
                let (r, t) = (camera_rotation, camera_translation);
                r * anchor_view_space + t
            };

            let rot_init = look_to(*initial_facing_direction, *initial_up_direction);
            let rot_target = look_to(*target_facing_direction, *target_up_direction);

            let rot_next = rot_init.slerp(rot_target, progress as f64);
            let rot_last = camera_rotation;
            let rot_delta = rot_next * rot_last.inverse();

            let original_translation = camera_translation;
            let original_rotation = camera_rotation;
            rotate_around(
                (&mut camera_translation, &mut camera_rotation),
                anchor_world,
                rot_delta,
            );
            let (_, delta_rotation, delta_translation) = {
                let original =
                    DAffine3::from_rotation_translation(original_rotation, original_translation);
                let new = DAffine3::from_rotation_translation(camera_rotation, camera_translation);
                (original.inverse() * new).to_scale_rotation_translation()
            };

            let mut camera_muts = camera_set.p2();
            let mut camera_mut = camera_muts.get_mut(*camera).unwrap();
            transform_adapter.apply_delta(&mut camera_mut, delta_translation, delta_rotation);
            if progress_t >= 1.0 {
                *complete = true;
            }
            redraw.write(RequestRedraw);
        }
        state.map.retain(|_, v| !v.complete);
    }
}
