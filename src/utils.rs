use bevy_app::{AppExit, Events};
use bevy_ecs::{Local, ResMut};

#[derive(Default)]
pub struct State {
    pub cnt: u32,
}

pub fn quit_after_ten_frames(
    mut state: Local<State>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    log::info!(
        "############################### frame #{} #################################",
        state.cnt
    );
    state.cnt += 1;
    if state.cnt >= 10 {
        app_exit_events.send(AppExit);
    }
}
