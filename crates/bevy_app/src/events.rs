use bevy_ecs::storage::{ThreadLocalTask, ThreadLocalTaskSendError, ThreadLocalTaskSender};
use std::any::Any;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Events an [`App`](crate::App) can send to another thread when using a multi-threaded runner.
pub enum AppEvent {
    /// The app has sent a task with access to [`ThreadLocals`](bevy_ecs::prelude::ThreadLocals).
    Task(ThreadLocalTask),
    /// The app has exited.
    Exit(Box<crate::SubApps>),
    /// The app has errored.
    Error(Box<dyn Any + Send>),
}

/// The sender half of an [`app_thread_channel`].
#[derive(Clone)]
pub struct AppEventSender(Sender<AppEvent>);

/// Constructs a new asynchronous channel for passing [`AppEvent`] instances
/// to an event loop and returns the sender and receiver halves.
pub fn app_thread_channel() -> (AppEventSender, AppEventReceiver) {
    let (send, recv) = channel();
    (AppEventSender(send), AppEventReceiver(recv))
}

impl std::ops::Deref for AppEventSender {
    type Target = Sender<AppEvent>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AppEventSender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ThreadLocalTaskSender for AppEventSender {
    fn send_task(
        &mut self,
        task: ThreadLocalTask,
    ) -> Result<(), ThreadLocalTaskSendError<ThreadLocalTask>> {
        self.send(AppEvent::Task(task)).map_err(|error| {
            let AppEvent::Task(task) = error.0 else {
                unreachable!()
            };
            ThreadLocalTaskSendError(task)
        })
    }
}

/// The receiver-half of an [`app_thread_channel`].
pub struct AppEventReceiver(Receiver<AppEvent>);

impl std::ops::Deref for AppEventReceiver {
    type Target = Receiver<AppEvent>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AppEventReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
