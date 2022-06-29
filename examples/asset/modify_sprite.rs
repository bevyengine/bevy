use bevy::prelude::*;

struct BevyLogoLight {
    handle: Handle<Image>,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(change_texture)
        .add_system(change_color)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("branding/bevy_logo_dark.png");
    let texture_handle_2: Handle<Image> = asset_server.load("branding/bevy_logo_light.png");
    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(SpriteBundle {
            texture: texture_handle.clone(),
            transform: Transform {
                translation: Vec3::new(1., 1., 1.),
                scale: Vec3::ONE,
                ..Default::default()
            },
            sprite: Sprite {
                color: Color::WHITE,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(AnimationTimer(Timer::from_seconds(1f32, false)));

    commands.insert_resource(BevyLogoLight {
        handle: texture_handle_2,
    });
}

fn change_texture(
    time: Res<Time>,
    bevy_logo_light: Res<BevyLogoLight>,
    mut query: Query<(&mut AnimationTimer, &mut Handle<Image>)>,
) {
    for (mut timer, mut handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            *handle = bevy_logo_light.handle.clone();
        }
    }
}

fn change_color(mut query: Query<(&mut AnimationTimer, &mut Sprite)>) {
    let (timer, mut sprite) = query.iter_mut().next().unwrap();
    if timer.finished() {
        sprite.color = Color::RED;
    }
}
