use bevy::app::{App, DeclarativePlugin};
use bevy_ecs::system::Query;

#[derive(Debug, Default)]
pub struct PluginA;

impl DeclarativePlugin for PluginA {
    fn build(&self, output: &mut bevy::app::PluginOutput) {}
}

#[derive(Debug, Default)]
pub struct PluginB;

impl DeclarativePlugin for PluginB {
    fn build(&self, output: &mut bevy::app::PluginOutput) {
        output.add_dependency_no_worries::<PluginA>();
    }
}

fn main() {
    let app = App::new();
}
