use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
    utils::Duration,
};
use std::num::Wrapping;

/// This example illustrates how to mutate a texture on the CPU.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_startup_system(setup_timer.system())
        .add_system(timer_tick.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Create a texture with varying shades of red.
    let texture = Texture::new_fill(
        Extent3d {
            width: 16,
            height: 16,
            depth: 1,
        },
        TextureDimension::D2,
        &(0..(256))
            .flat_map(|i| vec![255, i as u8, 0, 255])
            .collect::<Vec<u8>>(),
        TextureFormat::Rgba8UnormSrgb,
    );

    let texture_handle = textures.add(texture);

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle.clone().into()),
        transform: Transform::from_scale(Vec3::splat(30.0)),
        ..Default::default()
    });
    commands.insert_resource(texture_handle);
}

struct TickTrack {
    count: Wrapping<u8>,
    timer: Timer,
}

fn setup_timer(mut commands: Commands) {
    commands.insert_resource(TickTrack {
        count: Wrapping(0),
        timer: Timer::new(Duration::from_secs(1), true),
    });
}

fn timer_tick(
    time: Res<Time>,
    mut timer: ResMut<TickTrack>,
    mut textures: ResMut<Assets<Texture>>,
    pic: Res<Handle<Texture>>,
) {
    timer.timer.tick(time.delta());
    if timer.timer.finished() {
        timer.count += Wrapping(1);

        let texture = textures.get_mut(&*pic).unwrap();
        let idx = (timer.count.0 as usize) * 4 % texture.size.volume();
        // Each timer interval, one pixel turns black.
        texture.data[idx..(idx + 3)].iter_mut().for_each(|i| *i = 0);
    }
}
