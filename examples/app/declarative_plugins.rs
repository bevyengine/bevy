use bevy::app::{App, DeclarativePlugin};
use bevy_ecs::{resource::Resource, system::Query};

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
        output.insert_resource(MyResource("Cool".into()));
    }
}

#[derive(Debug, Resource)]
pub struct MyResource(String);

fn main() {
    let app = App::new();
}
