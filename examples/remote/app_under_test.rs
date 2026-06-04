//! A Bevy app that can be used as an integration test target.
//! It displays a button that must be clicked. The button is placed at a random position and
//! moves every 5 seconds.
//!
//! Run with the `bevy_remote` feature enabled:
//! ```bash
//! cargo run --example app_under_test --features="bevy_remote"
//! ```
//! This example can be paired with the `integration_test` example, which will run an integration
//! test on this app.

use bevy::{
    prelude::*,
    remote::{http::RemoteHttpPlugin, RemotePlugin},
    time::common_conditions::on_timer,
    ui::UiGlobalTransform,
};
use chacha20::ChaCha8Rng;
use rand::{RngExt, SeedableRng};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // To make the app available for integration testing, we add these
        // remote plugins to expose API’s for a testing framework to call.
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .insert_resource(SeededRng(ChaCha8Rng::seed_from_u64(19878367467712)))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_button.run_if(on_timer(std::time::Duration::from_secs(5))),
                log_button_position,
            ),
        )
        .run();
}

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

fn on_button_click(_click: On<Pointer<Click>>, mut exit: MessageWriter<AppExit>) {
    info!("Button pressed!");
    exit.write(AppExit::Success);
}

fn log_button_position(
    transform: Single<&UiGlobalTransform, (With<Button>, Changed<UiGlobalTransform>)>,
) {
    info!(
        "Button at physical ({}, {})",
        transform.translation.x, transform.translation.y
    );
}

fn random_position(rng: &mut ChaCha8Rng) -> (f32, f32) {
    let left_pct = rng.random_range(0.0..=60.0);
    let top_pct = rng.random_range(0.0..=60.0);
    (left_pct, top_pct)
}

fn move_button(mut rng: ResMut<SeededRng>, mut button_query: Query<&mut Node, With<Button>>) {
    let (left_pct, top_pct) = random_position(&mut rng.0);
    for mut node in &mut button_query {
        node.left = percent(left_pct);
        node.top = percent(top_pct);
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, mut rng: ResMut<SeededRng>) {
    let (left_pct, top_pct) = random_position(&mut rng.0);

    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: px(150),
                        height: px(65),
                        border: UiRect::all(px(5)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::MAX,
                        left: percent(left_pct),
                        top: percent(top_pct),
                        ..default()
                    },
                    BorderColor::all(Color::WHITE),
                    BackgroundColor(Color::BLACK),
                ))
                .observe(on_button_click)
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Button"),
                        TextFont {
                            font: assets.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(33.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextShadow::default(),
                    ));
                });
        });
}
