use bevy::{core::FixedTimestep, prelude::*};
use rand::Rng;

#[derive(Default)]
struct BevyMaterial(Option<Handle<ColorMaterial>>);

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
        .init_resource::<BevyMaterial>()
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bevy_material: ResMut<BevyMaterial>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let texture_handle = asset_server.load("branding/icon.png");
    let material = materials.add(texture_handle.into());
    bevy_material.0 = Some(material);
}

fn dynamic(mut commands: Commands, mut system_counter: Local<u64>) {
    let count = *system_counter;
    *system_counter += 1;
    let closure = move |mut commands: Commands, bevy_material: Res<BevyMaterial>| {
        info!("Hello from system {}", count);

        let mut rng = rand::thread_rng();

        commands.spawn_bundle(SpriteBundle {
            material: bevy_material.0.clone().unwrap(),
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
