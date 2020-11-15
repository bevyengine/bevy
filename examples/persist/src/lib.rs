use bevy::{persist::RestoreResource, prelude::*, winit::WinitConfig};

#[no_mangle]
pub fn __bevy_the_game(mut app: AppBuilder) {
    println!("loaded");
    app.add_resource(WinitConfig {
        return_from_run: true,
    })
    .add_plugins(DefaultPlugins)
    .add_serde_restore_resource(Scoreboard { score: 0 })
    .add_raw_restore_resource(Scoreboard2 { score: 0 })
    .add_startup_system(setup.system())
    .add_system(click_handler.system())
    .run();
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Scoreboard {
    score: usize,
}

struct Scoreboard2 {
    score: usize,
}

struct OurText;

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        // 2d camera
        .spawn(CameraUiBundle::default())
        // texture
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                value: "FPS:".to_string(),
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            },
            ..Default::default()
        })
        .with(OurText);
}

fn click_handler(
    key_input: Res<Input<KeyCode>>,
    mut score: ResMut<Scoreboard>,
    mut score2: ResMut<Scoreboard2>,
    mut query: Query<&mut Text, With<OurText>>,
) {
    if key_input.just_pressed(KeyCode::Space) {
        score.score += 1;
        score2.score += 1;
        for mut text in query.iter_mut() {
            text.value = format!("a{},{}", score.score, score2.score);
        }
    }
}
