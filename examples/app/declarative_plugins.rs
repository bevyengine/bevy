use bevy::app::DeclarativePlugin;
use bevy_ecs::system::Query;

pub struct PluginA;

impl DeclarativePlugin for PluginA {
    fn build(&self, output: &mut bevy::app::PluginOutput) {}
}

pub struct PluginB;

impl DeclarativePlugin for PluginB {
    fn build(&self, output: &mut bevy::app::PluginOutput) {}
}

fn main() {}
