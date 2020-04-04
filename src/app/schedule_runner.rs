use crate::{
    app::{App, AppBuilder},
    prelude::AppPlugin,
};
use std::{thread, time::Duration};

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

#[derive(Default)]
pub struct ScheduleRunner {
    pub run_mode: RunMode,
}

impl AppPlugin for ScheduleRunner {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        let run_mode = self.run_mode;
        app.set_runner(move |mut app: App| match run_mode {
            RunMode::Once => {
                app.schedule.execute(&mut app.world, &mut app.resources);
            }
            RunMode::Loop { wait } => loop {
                app.schedule.execute(&mut app.world, &mut app.resources);
                if let Some(wait) = wait {
                    thread::sleep(wait);
                }
            },
        })
    }
    fn name(&self) -> &'static str {
        "ScheduleRun"
    }
}
