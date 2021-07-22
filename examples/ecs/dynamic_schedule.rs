use bevy::{core::FixedTimestep, prelude::*};
use rand::Rng;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_stage_after(
            CoreStage::Update,
            "slow",
            SystemStage::parallel().with_run_criteria(FixedTimestep::step(1.0)),
        )
        .add_startup_system(setup)
        .add_system(dynamic.with_run_criteria(FixedTimestep::step(1.0)))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn dynamic(mut commands: Commands, mut system_counter: Local<u64>) {
    let count = *system_counter;
    *system_counter += 1;
    let closure = move |mut commands: Commands,
                        asset_server: Res<AssetServer>,
                        mut materials: ResMut<Assets<ColorMaterial>>| {
        info!("Hello from system {}", count);

        let mut rng = rand::thread_rng();
        let texture_handle = asset_server.load("branding/icon.png");
        commands.spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_xyz(
                rng.gen_range(-400f32..400f32),
                rng.gen_range(-400f32..400f32),
                0.0,
            ),
            ..Default::default()
        });
    };
    commands.insert_system(closure, "slow");
}
