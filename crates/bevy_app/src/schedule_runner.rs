use super::{App, AppBuilder};
use crate::{
    app::AppExit,
    event::{EventReader, Events},
    plugin::Plugin,
};
use std::{
    thread,
    time::{Duration, Instant},
};

/// Determines the method used to run an [App]'s `Schedule`
#[derive(Copy, Clone, Debug)]
pub enum RunMode {
    Loop { wait: Option<Duration> },
    Once,
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Loop { wait: None }
    }
}

/// Configures an App to run its [Schedule](bevy_ecs::Schedule) according to a given [RunMode]
#[derive(Default)]
pub struct ScheduleRunnerPlugin {
    pub run_mode: RunMode,
}

impl ScheduleRunnerPlugin {
    pub fn run_once() -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Once,
        }
    }

    pub fn run_loop(wait_duration: Duration) -> Self {
        ScheduleRunnerPlugin {
            run_mode: RunMode::Loop {
                wait: Some(wait_duration),
            },
        }
    }
}

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let run_mode = self.run_mode;
        app.set_runner(move |mut app: App| {
            let mut app_exit_event_reader = EventReader::<AppExit>::default();
            match run_mode {
                RunMode::Once => {
                    app.schedule.run(&mut app.world, &mut app.resources);
                }
                RunMode::Loop { wait } => loop {
                    let start_time = Instant::now();

                    if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
                        if app_exit_event_reader.latest(&app_exit_events).is_some() {
                            break;
                        }
                    }

                    app.schedule.run(&mut app.world, &mut app.resources);

                    if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
                        if app_exit_event_reader.latest(&app_exit_events).is_some() {
                            break;
                        }
                    }

                    let end_time = Instant::now();

                    if let Some(wait) = wait {
                        let exe_time = end_time - start_time;
                        if exe_time < wait {
                            thread::sleep(wait - exe_time);
                        }
                    }
                },
            }
        });
    }
}
