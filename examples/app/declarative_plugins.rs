use bevy::{DefaultPlugins, app::{App, DeclarativePlugin, Startup}, camera::{Camera2d, Camera3d}};
use bevy_ecs::{resource::Resource, system::{Commands, Query}};
use bevy_scene::{CommandsSceneExt, SceneList, SpawnListSystem, bsn_list};

#[derive(Debug, Default)]
pub struct PluginA;

impl DeclarativePlugin for PluginA {
    fn build(&self, output: &mut bevy::app::PluginOutput) {
        output.add_systems(Startup, startup.spawn());
    }
}

fn startup() -> impl SceneList {
    bsn_list!(
        Camera2d,
    )
}

#[derive(Debug, Default)]
pub struct PluginB;

impl DeclarativePlugin for PluginB {
    fn build(&self, output: &mut bevy::app::PluginOutput) {
        output.add_dependency::<PluginA>();
        output.require_resource_with_value(MyResource("Cool".into()));
    }
}

#[derive(Debug, Default)]
pub struct PluginC;

impl DeclarativePlugin for PluginC {
    fn build(&self, output: &mut bevy::app::PluginOutput) {
        output.add_dependency::<PluginA>();
        output.add_dependency::<PluginB>();
        output.require_resource_with_approval::<MyResource>(|res| res.0 != "Cool!");
    }
}

#[derive(Debug, Default)]
pub struct ConfigurablePlugin {
    pub config: bool,
}

#[derive(Debug, Resource, Default)]
pub struct MyResource(String);

fn main() {
    let app = App::new().add_plugins(DefaultPlugins);
}
