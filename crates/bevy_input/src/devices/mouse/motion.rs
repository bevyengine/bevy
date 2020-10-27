use crate::core::{ElementState, Input};
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use glam::f32::vec2::Vec2;

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}
