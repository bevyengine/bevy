use bevy_app::prelude::*;
use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::{Quat, Vec3};
use bevy_openxr_core::{event::XRState, hand_tracking::HandPoseState};
use bevy_pbr::{prelude::*, PbrBundle};
use bevy_render::prelude::*;
use bevy_transform::prelude::*;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Default)]
pub struct OpenXRHandTrackingPlugin;

impl Plugin for OpenXRHandTrackingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<HandTrackingState>()
            .add_startup_system(setup.system())
            .add_system(hand_visibility_system.system())
            .add_system(hand_system.system());
    }
}

struct LeftHand(usize);
struct RightHand(usize);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // https://www.khronos.org/registry/OpenXR/specs/1.0/html/xrspec.html
    // "Conventions of hand joints"

    // FIXME add parent objects

    // left hand
    for i in 0..openxr::HAND_JOINT_COUNT {
        commands
            .spawn_bundle(get_joint_box(i, &mut meshes, &mut materials))
            .insert(LeftHand(i));
    }

    // right hand
    for i in 0..openxr::HAND_JOINT_COUNT {
        commands
            .spawn_bundle(get_joint_box(i, &mut meshes, &mut materials))
            .insert(RightHand(i));
    }
}

fn get_joint_box(
    joint: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> PbrBundle {
    let size = match FromPrimitive::from_usize(joint).unwrap() {
        HandJoint::ThumbTip
        | HandJoint::IndexTip
        | HandJoint::MiddleTip
        | HandJoint::RingTip
        | HandJoint::LittleTip => 0.018 / 4.0,
        _ => 0.018,
    };

    // FIXME could have only two instances of mesh?
    PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size })),
        material: materials.add(Color::rgb(0., 0.7, 0.).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    }
}

#[derive(Default)]
struct HandTrackingState {
    visible: bool, // both, from upstream
    left_visible: bool,
    right_visible: bool,
}

fn hand_visibility_system(
    mut hand_tracking_state: ResMut<HandTrackingState>,
    mut xr_state_events: EventReader<XRState>,
    mut hand_boxes: QuerySet<(
        Query<(&mut Transform, &LeftHand, &mut Visible), With<LeftHand>>,
        Query<(&mut Transform, &RightHand, &mut Visible), With<RightHand>>,
    )>,
) {
    for state_event in xr_state_events.iter() {
        let visible = match state_event {
            XRState::RunningFocused => true,
            XRState::Paused | XRState::Exiting | XRState::Running => false,
        };

        // println!("Change hands visibility to {}", visible);

        hand_tracking_state.visible = visible;
        hand_tracking_state.left_visible = visible;
        hand_tracking_state.right_visible = visible;

        for (_, _, mut visible) in hand_boxes.q0_mut().iter_mut() {
            visible.is_visible = hand_tracking_state.visible;
        }

        for (_, _, mut visible) in hand_boxes.q1_mut().iter_mut() {
            visible.is_visible = hand_tracking_state.visible;
        }
    }
}

fn hand_system(
    hand_pose: Res<HandPoseState>,
    mut hand_tracking_state: ResMut<HandTrackingState>,
    mut hand_boxes: QuerySet<(
        Query<(&mut Transform, &LeftHand, &mut Visible), With<LeftHand>>,
        Query<(&mut Transform, &RightHand, &mut Visible), With<RightHand>>,
    )>,
) {
    if !hand_tracking_state.visible {
        return;
    }

    if let Some(left) = hand_pose.left {
        if !hand_tracking_state.left_visible {
            for (_, _, mut visible) in hand_boxes.q0_mut().iter_mut() {
                visible.is_visible = true;
            }
            hand_tracking_state.left_visible = true;
        }

        for (mut hand, idx, _) in hand_boxes.q0_mut().iter_mut() {
            let pos = &left[idx.0].pose.position;
            let ori = &left[idx.0].pose.orientation;
            hand.translation = Vec3::new(pos.x, pos.y, pos.z);
            hand.rotation = Quat::from_xyzw(ori.x, ori.y, ori.z, ori.w);

            /*
            let flags = left[idx.0].location_flags;
            //flags.contains...

            if flags.contains(SpaceLocationFlags::POSITION_VALID) {
                hand.scale.x = 1.0;
                hand.scale.y = 1.0;
                hand.scale.z = 1.0;
            } else {
                hand.scale.x = 0.5;
                hand.scale.y = 0.5;
                hand.scale.z = 0.5;
            }
             */
        }
    } else {
        for (_, _, mut visible) in hand_boxes.q0_mut().iter_mut() {
            visible.is_visible = false;
        }
        hand_tracking_state.left_visible = false;
    }

    if let Some(right) = hand_pose.right {
        if !hand_tracking_state.right_visible {
            for (_, _, mut visible) in hand_boxes.q1_mut().iter_mut() {
                visible.is_visible = true;
            }
            hand_tracking_state.right_visible = true;
        }

        for (mut hand, idx, _) in hand_boxes.q1_mut().iter_mut() {
            let pos = &right[idx.0].pose.position;
            let ori = &right[idx.0].pose.orientation;
            hand.translation = Vec3::new(pos.x, pos.y, pos.z);
            hand.rotation = Quat::from_xyzw(ori.x, ori.y, ori.z, ori.w);
        }
    } else {
        for (_, _, mut visible) in hand_boxes.q1_mut().iter_mut() {
            visible.is_visible = false;
        }
        hand_tracking_state.right_visible = false;
    }
}

// https://www.khronos.org/registry/OpenXR/specs/1.0/html/xrspec.html
// typedef enum XrHandJointEXT
#[derive(FromPrimitive)]
enum HandJoint {
    Palm = 0,
    Wrist = 1,
    ThumbMetacarpal = 2,
    ThumbProximal = 3,
    ThumbDistal = 4,
    ThumbTip = 5,
    IndexMetacarpal = 6,
    IndexProximal = 7,
    IndexIntermediate = 8,
    IndexDistal = 9,
    IndexTip = 10,
    MiddleMetacarpal = 11,
    MiddleProximal = 12,
    MiddleIntermediate = 13,
    MiddleDistal = 14,
    MiddleTip = 15,
    RingMetacarpal = 16,
    RingProximal = 17,
    RingIntermediate = 18,
    RingDistal = 19,
    RingTip = 20,
    LittleMetacarpal = 21,
    LittleProximal = 22,
    LittleIntermediate = 23,
    LittleDistal = 24,
    LittleTip = 25,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a() {
        //let x = openxr::sys::SpaceLocationFlags::from_raw(openxr::sys::SpaceLocationFlags::ORIENTATION_VALID.into_raw());
        //assert_eq!(x.intersects(openxr::sys::SpaceLocationFlags::ORIENTATION_VALID), true);
    }
}

/*
pub struct SpaceLocationFlags(u64);
impl SpaceLocationFlags {
    #[doc = "Indicates validity of orientation member"]
    pub const ORIENTATION_VALID: SpaceLocationFlags = Self(1 << 0u64);
    #[doc = "Indicates validity of position member"]
    pub const POSITION_VALID: SpaceLocationFlags = Self(1 << 1u64);
    #[doc = "Indicates whether pose member contains an actively tracked orientation"]
    pub const ORIENTATION_TRACKED: SpaceLocationFlags = Self(1 << 2u64);
    #[doc = "Indicates whether pose member contains an actively tracked position"]
    pub const POSITION_TRACKED: SpaceLocationFlags = Self(1 << 3u64);
}
*/
