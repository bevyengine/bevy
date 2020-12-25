use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(animate.system())
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        // 2d camera
        .spawn(Camera2dBundle::default())
        .spawn(Text2dBundle {
            text: Text {
                value: "This text is in the 2D scene.".to_string(),
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                    alignment: TextAlignment {
                        vertical: VerticalAlign::Center,
                        horizontal: HorizontalAlign::Center,
                    },
                },
            },
            ..Default::default()
        });
}

fn animate(time: Res<Time>, mut query: Query<&mut Transform, With<Text>>) {
    // `Transform.translation` will determine the location of the text.
    // `Transform.scale` and `Transform.rotation` do not yet affect text (though you can set the
    // size of the text via `Text.style.font_size`)
    for mut transform in query.iter_mut() {
        transform.translation.x = 100.0 * time.seconds_since_startup().sin() as f32;
        transform.translation.y = 100.0 * time.seconds_since_startup().cos() as f32;
    }
}
