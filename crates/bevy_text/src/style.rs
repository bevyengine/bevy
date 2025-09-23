use crate::*;
use bevy_app::Propagate;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;
use bevy_ecs::query::AnyOf;

#[derive(Resource)]
pub struct DefaultTextStyle {
    font: TextFont,
    color: Color,
}

#[derive(Component)]
pub struct ComputedTextStyle {
    font: TextFont,
    color: Color,
}
