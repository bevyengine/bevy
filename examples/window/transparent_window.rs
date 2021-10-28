use bevy::{
    prelude::*,
    render::pass::ClearColor,
    window::WindowDescriptor,
};

fn main() {
    App::new()
        // rgba value needs to be [0, 0, 0, 0], otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(WindowDescriptor {
            // setting transparent allows the window to become transparent when clear color has the correct value
            transparent: true,
            // Disabling window desoration to make it feel more like a widget than a window
            decorations: false,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle.into()),
        ..Default::default()
    });
}