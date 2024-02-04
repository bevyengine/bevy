//! This example shows how to use the ECS and the [`AsyncComputeTaskPool`]
//! to spawn, poll, and complete web request.

use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use ehttp::{Request, Response};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ApiTimer>()
        .add_systems(Startup, setup_env)
        .add_systems(
            Update,
            (send_request, handle_request, handle_response, update_text),
        )
        .run();
}

/// wrap for ehttp request
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct HttpRequest(pub Request);

/// wrap for ehttp response
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct HttpResponse(pub Response);

/// wrap for ehttp error
#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct HttpResponseError(pub String);

/// task for ehttp response result
#[derive(Component)]
pub struct RequestTask(pub Task<Result<Response, ehttp::Error>>);

/// timer component
#[derive(Resource, Deref, DerefMut)]
pub struct ApiTimer(pub Timer);

impl Default for ApiTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Repeating))
    }
}

/// system that will tick timer and send request each time timer has just finished
fn send_request(mut commands: Commands, time: Res<Time>, mut timer: ResMut<ApiTimer>) {
    timer.tick(time.delta());

    if timer.just_finished() {
        let req = ehttp::Request::get("https://api.ipify.org?format=json");
        commands.spawn(HttpRequest(req));
    }
}

/// system that will spawn task for each request
fn handle_request(
    mut commands: Commands,
    requests: Query<(Entity, &HttpRequest), Without<RequestTask>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    for (entity, request) in requests.iter() {
        let req = request.clone();

        let s = thread_pool.spawn(async { ehttp::fetch_async(req.0).await });

        commands
            .entity(entity)
            .remove::<HttpRequest>()
            .insert(RequestTask(s));
    }
}

/// system that will detect when request task has finished and parse the result
fn handle_response(mut commands: Commands, mut request_tasks: Query<(Entity, &mut RequestTask)>) {
    for (entity, mut task) in request_tasks.iter_mut() {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            match result {
                Ok(res) => {
                    commands
                        .entity(entity)
                        .insert(HttpResponse(res))
                        .remove::<RequestTask>();
                }
                Err(e) => {
                    commands
                        .entity(entity)
                        .insert(HttpResponseError(e))
                        .remove::<RequestTask>();
                }
            }
        }
    }
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
    started_request: Query<&HttpRequest, Added<HttpRequest>>,
    success_q: Query<(Entity, &HttpResponse)>,
    failed_q: Query<(Entity, &HttpResponseError)>,
    mut text_query: Query<&mut Text>,
) {
    let text = text_query.get_single_mut();
    if text.is_err() {
        return;
    }
    let mut text = text.unwrap();
    for request in started_request.iter() {
        text.sections[0].value = format!("Request started:\n{}", request.url);
        text.sections[0].style.color = Color::WHITE;
    }
    for (entity, response) in success_q.iter() {
        text.sections[0].value = format!("Request response:\n{}", response.text().unwrap());
        text.sections[0].style.color = Color::DARK_GREEN;
        commands.entity(entity).despawn_recursive();
    }
    for (entity, error) in failed_q.iter() {
        text.sections[0].value = format!("Request failed:\n{}", error.0);
        text.sections[0].style.color = Color::RED;
        commands.entity(entity).despawn_recursive();
    }
}
