use bevy_app::{App, Plugin};
use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Button;

/// A plugin for button widgets
#[derive(Default)]
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Button>();
    }
}
