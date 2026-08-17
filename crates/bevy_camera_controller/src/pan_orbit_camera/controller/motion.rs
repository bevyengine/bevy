//! The motion state of the camera.

use std::time::Duration;

use super::{inputs::MotionInputs, momentum::Velocity};
use bevy_math::DVec3;
use bevy_platform::time::Instant;

/// The current motion state of the camera.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum CurrentMotion {
    /// The camera is not moving.
    #[default]
    Stationary,
    /// The camera is in motion, but not being directly controlled by the user. This happens while
    /// the camera has momentum.
    Momentum {
        /// Contains inherited velocity, if any. This will decay based on momentum settings.
        velocity: Velocity,
        /// Used to compute how long the camera has been in the momentum state. Useful for
        /// debouncing user inputs.
        momentum_start: Instant,
    },
    /// The camera is being directly controlled by the user.
    UserControlled {
        /// The point the camera is rotating about, zooming into, or panning with, in view space
        /// (relative to the camera).
        ///
        /// - Rotation: the direction of the anchor does not change, it is fixed in screenspace.
        /// - Panning: the depth of the anchor does not change, the camera only moves in x and y.
        /// - Zoom: the direction of the anchor does not change, but the length does.
        anchor: DVec3,
        /// Pan and orbit are mutually exclusive, however both can be used with zoom.
        motion_inputs: MotionInputs,
    },
}

impl CurrentMotion {
    /// Returns `true` if the camera is moving due to inputs or momentum.
    pub fn is_moving(&self) -> bool {
        !matches!(self, CurrentMotion::Stationary)
            && !matches!(
                self,
                CurrentMotion::Momentum {
                    velocity: Velocity::None,
                    ..
                }
            )
    }

    /// Returns `true` if the camera is moving due to user inputs.
    pub fn is_user_controlled(&self) -> bool {
        matches!(self, CurrentMotion::UserControlled { .. })
    }

    /// Get the user motion inputs if they exist.
    pub fn inputs(&self) -> Option<&MotionInputs> {
        match self {
            CurrentMotion::Stationary => None,
            CurrentMotion::Momentum { .. } => None,
            CurrentMotion::UserControlled { motion_inputs, .. } => Some(motion_inputs),
        }
    }

    /// Returns true if the camera is user controlled and orbiting.
    pub fn is_orbiting(&self) -> bool {
        matches!(
            self,
            Self::UserControlled {
                motion_inputs: MotionInputs::OrbitZoom { .. },
                ..
            }
        )
    }

    /// Returns true if the camera is user controlled and panning.
    pub fn is_panning(&self) -> bool {
        matches!(
            self,
            Self::UserControlled {
                motion_inputs: MotionInputs::PanZoom { .. },
                ..
            }
        )
    }

    /// Returns true if the camera is user controlled and only zooming.
    pub fn is_zooming_only(&self) -> bool {
        matches!(
            self,
            Self::UserControlled {
                motion_inputs: MotionInputs::Zoom { .. },
                ..
            }
        )
    }

    /// How long has the camera been moving with momentum, without user input? This is equivalent to
    /// the amount of time since the last input event ended.
    pub fn momentum_duration(&self) -> Option<Duration> {
        match self {
            CurrentMotion::Momentum { momentum_start, .. } => {
                Some(Instant::now().saturating_duration_since(*momentum_start))
            }
            _ => None,
        }
    }
}
