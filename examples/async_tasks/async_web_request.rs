//! This example shows how to use the ECS and the [`AsyncComputeTaskPool`]
//! to spawn, poll, and complete web request.

use std::time::Duration;

use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
    time::common_conditions::on_timer,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_env)
        .add_systems(Update, update_text)
        .add_systems(
            Update,
            send_request.run_if(on_timer(Duration::from_secs(3))),
        )
        .run();
}

/// Task for ehttp response result
#[derive(Component)]
pub struct RequestTask(pub Task<Result<ehttp::Response, ehttp::Error>>);

/// Tick timer and send request each time the timer has just finished
fn send_request(mut commands: Commands, mut text_query: Query<&mut Text>) {
    let url = "https://api.ipify.org?format=json";
    let req = ehttp::Request::get(url);
    let thread_pool = IoTaskPool::get();
    let s = thread_pool.spawn(async { ehttp::fetch_async(req).await });
    commands.spawn(RequestTask(s));
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };
    text.sections[0].value = format!("Request started:\n{}", url);
    text.sections[0].style.color = Color::WHITE;
}

/// This system is used to setup text and camera for the environment
fn setup_env(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 30.0,
        color: Color::WHITE,
    };
    let text_justification = JustifyText::Center;
    // 2d camera
    commands.spawn(Camera2dBundle::default());
    commands.spawn(Text2dBundle {
        text: Text::from_section("Hi! Request will start in few seconds.", text_style.clone())
            .with_justify(text_justification),
        ..default()
    });
}

/// This system will update text based on requests progress
fn update_text(
    mut commands: Commands,
    mut text_query: Query<&mut Text>,
    mut request_tasks: Query<(Entity, &mut RequestTask)>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    for (entity, mut task) in request_tasks.iter_mut() {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            match result {
                Ok(response) => {
                    text.sections[0].value =
                        format!("Request response:\n{}", response.text().unwrap());
                    text.sections[0].style.color = Color::DARK_GREEN;
                }
                Err(error) => {
                    text.sections[0].value = format!("Request failed:\n{}", error);
                    text.sections[0].style.color = Color::RED;
                }
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}
