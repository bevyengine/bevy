//! Defines mutually exclusive camera input motions, and a place to store these input streams.

use std::time::Duration;

use super::smoothing::InputQueue;
use bevy_math::{prelude::*, DVec2};

/// Tracks the current exclusive motion type and input queue of the camera controller.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum MotionInputs {
    /// The camera can orbit and zoom
    OrbitZoom {
        /// A queue of screenspace orbiting inputs; usually the mouse drag vector.
        screenspace_inputs: InputQueue<Vec2>,
        /// A queue of zoom inputs.
        zoom_inputs: InputQueue<f32>,
    },
    /// The camera can pan and zoom
    PanZoom {
        /// A queue of screenspace panning inputs; usually the mouse drag vector.
        screenspace_inputs: InputQueue<Vec2>,
        /// A queue of zoom inputs.
        zoom_inputs: InputQueue<f32>,
    },
    /// The camera can only zoom
    Zoom {
        /// A queue of zoom inputs.
        zoom_inputs: InputQueue<f32>,
    },
}

impl MotionInputs {
    /// The motion-conserving smoothed orbit velocity in screen space.
    pub fn smooth_orbit_velocity(&self) -> DVec2 {
        if let Self::OrbitZoom {
            screenspace_inputs, ..
        } = self
        {
            let value = screenspace_inputs
                .latest_smoothed()
                .unwrap_or(Vec2::ZERO)
                .as_dvec2();
            if value.is_finite() {
                value
            } else {
                DVec2::ZERO
            }
        } else {
            DVec2::ZERO
        }
    }

    /// The motion-conserving smoothed pan velocity in screen space.
    pub fn smooth_pan_velocity(&self) -> DVec2 {
        if let Self::PanZoom {
            screenspace_inputs, ..
        } = self
        {
            let value = screenspace_inputs
                .latest_smoothed()
                .unwrap_or(Vec2::ZERO)
                .as_dvec2();
            if value.is_finite() {
                value
            } else {
                DVec2::ZERO
            }
        } else {
            DVec2::ZERO
        }
    }

    /// Approximate orbit velocity over the last `window`. to use for momentum calculations.
    pub fn orbit_momentum(&self, window: Duration) -> DVec2 {
        if let Self::OrbitZoom {
            screenspace_inputs, ..
        } = self
        {
            let velocity = screenspace_inputs.average_smoothed_value(window).as_dvec2();
            if !velocity.is_finite() {
                DVec2::ZERO
            } else {
                velocity
            }
        } else {
            DVec2::ZERO
        }
    }

    /// Approximate pan velocity over the last `window`. to use for momentum calculations.
    pub fn pan_momentum(&self, window: Duration) -> DVec2 {
        if let Self::PanZoom {
            screenspace_inputs, ..
        } = self
        {
            let velocity = screenspace_inputs.average_smoothed_value(window).as_dvec2();
            if !velocity.is_finite() {
                DVec2::ZERO
            } else {
                velocity
            }
        } else {
            DVec2::ZERO
        }
    }

    /// Motion-conserving smoothed zoom input velocity.
    pub fn smooth_zoom_velocity(&self) -> f64 {
        let velocity = self.zoom_inputs().latest_smoothed().unwrap_or(0.0) as f64;
        if !velocity.is_finite() {
            0.0
        } else {
            velocity
        }
    }

    /// Get a reference to the queue of zoom inputs.
    pub fn zoom_inputs(&self) -> &InputQueue<f32> {
        match self {
            MotionInputs::OrbitZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::PanZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::Zoom { zoom_inputs } => zoom_inputs,
        }
    }

    /// Get a mutable reference to the queue of zoom inputs.
    pub fn zoom_inputs_mut(&mut self) -> &mut InputQueue<f32> {
        match self {
            MotionInputs::OrbitZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::PanZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::Zoom { zoom_inputs } => zoom_inputs,
        }
    }

    /// Approximate smoothed  absolute value of the zoom velocity over the last `window`.
    pub fn zoom_velocity_abs(&self, window: Duration) -> f64 {
        let zoom_inputs = match self {
            MotionInputs::OrbitZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::PanZoom { zoom_inputs, .. } => zoom_inputs,
            MotionInputs::Zoom { zoom_inputs } => zoom_inputs,
        };

        let velocity = zoom_inputs.approx_smoothed(window, |v| {
            *v = v.abs();
        }) as f64;
        if !velocity.is_finite() {
            0.0
        } else {
            velocity
        }
    }
}
