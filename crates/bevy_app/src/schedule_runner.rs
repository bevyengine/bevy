use super::{App, AppBuilder};
use crate::{
    app::AppExit,
    event::{EventReader, Events},
    plugin::Plugin,
};
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::{thread, time::Instant};

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};

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
                    app.update();
                }
                RunMode::Loop { wait } => {
                    let mut tick = move |app: &mut App,
                                         wait: Option<Duration>|
                          -> Result<Option<Duration>, AppExit> {
                        let start_time = Instant::now();

                        if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
                            if let Some(exit) = app_exit_event_reader.latest(&app_exit_events) {
                                return Err(exit.clone());
                            }
                        }

                        app.update();

                        if let Some(app_exit_events) = app.resources.get_mut::<Events<AppExit>>() {
                            if let Some(exit) = app_exit_event_reader.latest(&app_exit_events) {
                                return Err(exit.clone());
                            }
                        }

                        let end_time = Instant::now();

                        if let Some(wait) = wait {
                            let exe_time = end_time - start_time;
                            if exe_time < wait {
                                return Ok(Some(wait - exe_time));
                            }
                        }

                        Ok(None)
                    };

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        while let Ok(delay) = tick(&mut app, wait) {
                            if let Some(delay) = delay {
                                thread::sleep(delay);
                            }
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        fn set_timeout(f: &Closure<dyn FnMut()>, dur: Duration) {
                            web_sys::window()
                                .unwrap()
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    f.as_ref().unchecked_ref(),
                                    dur.as_millis() as i32,
                                )
                                .expect("should register `setTimeout`");
                        }
                        let asap = Duration::from_millis(1);

                        let mut rc = Rc::new(app);
                        let f = Rc::new(RefCell::new(None));
                        let g = f.clone();

                        let c = move || {
                            let mut app = Rc::get_mut(&mut rc).unwrap();
                            let delay = tick(&mut app, wait);
                            match delay {
                                Ok(delay) => {
                                    set_timeout(f.borrow().as_ref().unwrap(), delay.unwrap_or(asap))
                                }
                                Err(_) => {}
                            }
                        };
                        *g.borrow_mut() = Some(Closure::wrap(Box::new(c) as Box<dyn FnMut()>));
                        set_timeout(g.borrow().as_ref().unwrap(), asap);
                    };
                }
            }
        });
    }
}
