//! Demonstrates how to load and spawn BSN assets at runtime.

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::ecs::reflect::{ReflectFromTemplate, ReflectTemplate};
use bevy::light::CascadeShadowConfig;
use bevy::prelude::*;
use bevy_scene2::ScenePatchInstance;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_light_direction)
        .run();
}

#[derive(Clone, Copy, Default, Component, Debug, Reflect)]
#[reflect(Clone, Default, Component)]
enum TestEnum {
    #[default]
    Foo,
    Bar,
    Baz,
}

#[derive(Clone, Default, Component, Debug, Reflect)]
#[reflect(Default, Component)]
struct TestStruct {
    the_enum: TestEnum,
}

#[derive(Clone, Component, Debug, Reflect, FromTemplate)]
#[reflect(Clone, Component, FromTemplate)]
#[template(reflect)]
struct HandleContainer {
    field: Handle<Scene>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_patch = asset_server.load("scenes/example.bsn");
    commands.spawn(ScenePatchInstance(scene_patch));
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_secs() * PI / 5.0,
            -FRAC_PI_4,
        );
    }
}
