use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(change_texture)
        .add_system(change_color)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/bevy_logo_dark.png");
    let texture_handle_2: Handle<Texture> = asset_server.load("branding/bevy_logo_light.png");
    let handle = materials.add(texture_handle_2.into());
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        })
        .insert(Timer::from_seconds(1.5, false));

    commands.insert_resource(handle);
}

fn change_texture(
    time: Res<Time>,
    texture: Res<Handle<ColorMaterial>>,
    mut query: Query<(&mut Timer, &mut Handle<ColorMaterial>)>,
) {
    for (mut timer, mut handle) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            *handle = texture.clone();
        }
    }
}

fn change_color(
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(&mut Timer, &Handle<ColorMaterial>)>,
) {
    for (mut timer, handle) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            let material = materials.get_mut(handle).unwrap();
            material.color = Color::rgb(1.0, 0.0, 0.0);
        }
    }
}
