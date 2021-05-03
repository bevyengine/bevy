use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(animate.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 2d camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(Text2dBundle {
        text: Text::with_section(
            "This text is in the 2D scene.",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 60.0,
                color: Color::WHITE,
            },
            TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
        ),
        ..Default::default()
    });
}

fn animate(time: Res<Time>, mut query: Query<&mut Transform, With<Text>>) {
    // `Transform.translation` will determine the location of the text.
    // `Transform.scale` (though you can set the size of the text via
    // `Text.style.font_size`)
    for mut transform in query.iter_mut() {
        transform.translation.x = 100.0 * time.seconds_since_startup().sin() as f32;
        transform.translation.y = 100.0 * time.seconds_since_startup().cos() as f32;
        transform.rotation = Quat::from_rotation_z(time.seconds_since_startup().cos() as f32);
    }
}
