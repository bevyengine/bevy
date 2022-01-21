use bevy::prelude::*;

/// This example illustrates how to load and play an audio file
fn main() {
    App::new()
        .insert_resource(StopTimer(Timer::from_seconds(5.0, false)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(play_audio)
        .add_system(play_control)
        .run();
}

struct StopTimer(Timer);

fn play_control(time: Res<Time>, mut timer: ResMut<StopTimer>, mut ev: EventWriter<PlayEvent>) {
    if timer.0.tick(time.delta()).just_finished() {
        ev.send(PlayEvent::Clear)
    }
}

fn play_audio(res: Res<AssetServer>, mut ew: EventWriter<PlayEvent>) {
    let music = res.load("sounds/Windless Slopes.ogg");
    ew.send(PlayEvent::Loop(true));
    ew.send(PlayEvent::Append(music));
}
