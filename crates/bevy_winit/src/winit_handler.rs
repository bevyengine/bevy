use bevy_ecs::system::Resource;
use bevy_utils::{synccell::SyncCell, tracing::warn};
use winit::event_loop::{EventLoop, EventLoopProxy};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum HandleEvent {
    Run(f64),
    RunFullThrottle,
    Pause,
    Step(u64),
    RequestRedraw,
    DetermineRedraw,
    Exit(i32),
}

/// Controls the operation of the winit runner.
#[derive(Resource)]
pub struct WinitHandler {
    proxy: SyncCell<EventLoopProxy<HandleEvent>>,
    is_running: bool,
}

impl WinitHandler {
    pub(crate) fn new(event_loop: &EventLoop<HandleEvent>) -> Self {
        Self {
            proxy: SyncCell::new(event_loop.create_proxy()),
            is_running: false,
        }
    }

    /// Whether the ticks are automatically run or not.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Run game ticks in sync with real time.
    pub fn run(&mut self) {
        self.run_at(1.);
    }

    /// Run game ticks in sync with real time with a specified speed.
    pub fn run_at(&mut self, rate_multiplier: f64) {
        self.is_running = true;
        if let Err(e) = self
            .proxy
            .get()
            .send_event(HandleEvent::Run(rate_multiplier))
        {
            warn!(%e);
        }
    }

    /// Run game ticks as fast as possible.
    pub fn run_full_throttle(&mut self) {
        self.is_running = true;
        if let Err(e) = self.proxy.get().send_event(HandleEvent::RunFullThrottle) {
            warn!(%e);
        }
    }

    /// Stop automatic running of game ticks.
    pub fn pause(&mut self) {
        self.is_running = false;
        if let Err(e) = self.proxy.get().send_event(HandleEvent::Pause) {
            warn!(%e);
        }
    }

    /// Run a game tick only once.
    pub fn step(&mut self) {
        self.step_at(1);
    }

    /// Run game ticks a specified number of times.
    pub fn step_at(&mut self, request_steps: u64) {
        self.is_running = false;
        if let Err(e) = self
            .proxy
            .get()
            .send_event(HandleEvent::Step(request_steps))
        {
            warn!(%e);
        }
    }

    /// Requests frame redraw.
    /// 
    /// If called during redrawing, the next redraw is reserved.
    pub fn redraw(&mut self) {
        if let Err(e) = self.proxy.get().send_event(HandleEvent::RequestRedraw) {
            warn!(%e);
        }
    }

    /// Exit the application.
    pub fn exit(&mut self, code: i32) {
        if let Err(e) = self.proxy.get().send_event(HandleEvent::Exit(code)) {
            warn!(%e);
        }
    }
}
