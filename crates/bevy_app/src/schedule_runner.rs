use crate::{app_thread_channel, App, AppEvent, AppExit, Plugin, PluginsState, SubApps};
use bevy_ecs::event::{Events, ManualEventReader};
use bevy_utils::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};

/// Determines how frequently the [`App`] should be updated by the [`ScheduleRunnerPlugin`].
#[derive(Copy, Clone, Debug)]
pub enum RunMode {
    /// The [`App`] will update once.
    Once,
    /// The [`App`] will update over and over, until an [`AppExit`] event appears.
    Loop {
        /// The minimum time from the start of one update to the next.
        ///
        /// **Note:** This has no upper limit, but the [`App`] will hang if you set this too high.
        wait: Duration,
    },
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Loop {
            wait: Duration::ZERO,
        }
    }
}

/// Runs an [`App`] according to the selected [`RunMode`].
///
/// This plugin is included in the [`MinimalPlugins`] group, but **not** included in the
/// [`DefaultPlugins`] group. [`DefaultPlugins`] assumes the [`App`] will render to a window,
/// so it comes with the [`WinitPlugin`] instead.
///
/// [`DefaultPlugins`]: https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html
/// [`MinimalPlugins`]: https://docs.rs/bevy/latest/bevy/struct.MinimalPlugins.html
/// [`WinitPlugin`]: https://docs.rs/bevy/latest/bevy/winit/struct.WinitPlugin.html
#[derive(Default)]
pub struct ScheduleRunnerPlugin {
    /// Determines how frequently the [`App`] should update.
    pub run_mode: RunMode,
}

impl ScheduleRunnerPlugin {
    /// See [`RunMode::Once`].
    pub fn run_once() -> Self {
        Self {
            run_mode: RunMode::Once,
        }
    }

    /// See [`RunMode::Loop`].
    pub fn run_loop(wait: Duration) -> Self {
        Self {
            run_mode: RunMode::Loop { wait },
        }
    }
}

impl Plugin for ScheduleRunnerPlugin {
    fn build(&self, app: &mut App) {
        let run_mode = self.run_mode;
        app.set_runner(move |mut app: App| {
            // TODO: rework app setup
            // create channel
            let (send, recv) = app_thread_channel();
            // insert channel
            app.sub_apps.iter_mut().for_each(|sub_app| {
                app.tls.insert_channel(sub_app.world_mut(), send.clone());
            });

            // wait for plugins to finish setting up
            let plugins_state = app.plugins_state();
            if plugins_state != PluginsState::Cleaned {
                while app.plugins_state() == PluginsState::Adding {
                    #[cfg(not(target_arch = "wasm32"))]
                    bevy_tasks::tick_global_task_pools_on_main_thread();
                }
                app.finish();
                app.cleanup();
            }

            let mut exit_event_reader = ManualEventReader::<AppExit>::default();
            match run_mode {
                RunMode::Once => {
                    // if plugins where cleaned before the runner start, an update already ran
                    if plugins_state != PluginsState::Cleaned {
                        app.update();
                    }
                }
                RunMode::Loop { wait } => {
                    let mut update = move |sub_apps: &mut SubApps| -> Result<Duration, AppExit> {
                        let start_time = Instant::now();
                        sub_apps.update();
                        let end_time = Instant::now();

                        if let Some(exit_events) =
                            sub_apps.main.world().get_resource::<Events<AppExit>>()
                        {
                            if let Some(exit) = exit_event_reader.read(exit_events).last() {
                                return Err(exit.clone());
                            }
                        }

                        let elapsed = end_time - start_time;
                        if elapsed < wait {
                            return Ok(wait - elapsed);
                        }

                        Ok(Duration::ZERO)
                    };

                    // disassemble
                    let (mut sub_apps, _, _) = app.into_parts();

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // Move sub-apps to another thread and run an event loop in this thread.
                        let thread = std::thread::spawn(move || {
                            let result = catch_unwind(AssertUnwindSafe(|| {
                                while let Ok(sleep) = update(&mut sub_apps) {
                                    if !sleep.is_zero() {
                                        std::thread::sleep(sleep);
                                    }
                                }

                                send.send(AppEvent::Exit(sub_apps)).unwrap();
                            }));

                            if let Some(payload) = result.err() {
                                send.send(AppEvent::Error(payload)).unwrap();
                            }
                        });

                        loop {
                            let event = recv.recv().unwrap();
                            match event {
                                AppEvent::Task(task) => {
                                    task();
                                }
                                AppEvent::Exit(_) => {
                                    thread.join().unwrap();
                                    break;
                                }
                                AppEvent::Error(payload) => {
                                    resume_unwind(payload);
                                }
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
                                .expect("Should register `setTimeout`.");
                        }

                        let min_sleep = Duration::from_millis(1);

                        let mut rc = Rc::new(sub_apps);
                        let f = Rc::new(RefCell::new(None));
                        let g = f.clone();

                        let closure = move || {
                            let mut sub_apps = Rc::get_mut(&mut rc).unwrap();
                            match update(&mut sub_apps) {
                                Ok(sleep) => {
                                    set_timeout(f.borrow().as_ref().unwrap(), sleep.max(min_sleep))
                                }
                                Err(_) => {}
                            }
                        };

                        *g.borrow_mut() =
                            Some(Closure::wrap(Box::new(closure) as Box<dyn FnMut()>));

                        set_timeout(g.borrow().as_ref().unwrap(), min_sleep);
                    };
                }
            }
        });
    }
}
