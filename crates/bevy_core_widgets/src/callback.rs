use bevy_ecs::system::{Commands, SystemId, SystemInput};

/// A callback defines how we want to be notified when a widget changes state. Unlike an event
/// or observer, callbacks are intended for "point-to-point" communication that cuts across the
/// hierarchy of entities. Callbacks can be created in advance of the entity they are attached
/// to, and can be passed around as parameters.
#[derive(Default, Debug)]
pub enum Callback<I: SystemInput = ()> {
    /// Invoke a one-shot system
    System(SystemId<I>),
    /// Ignore this notification
    #[default]
    Ignore,
}

/// Trait to invoke callbacks
pub trait Notify {
    /// Invoke the callback with no arguments.
    fn notify(&mut self, callback: &Callback<()>);

    /// Invoke the callback with one argument.
    fn notify_arg<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static;
}

impl<'w, 's> Notify for Commands<'w, 's> {
    fn notify(&mut self, callback: &Callback<()>) {
        match callback {
            Callback::System(system_id) => self.run_system(*system_id),
            Callback::Ignore => (),
        }
    }

    fn notify_arg<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        match callback {
            Callback::System(system_id) => self.run_system_with(*system_id, input),
            Callback::Ignore => (),
        }
    }
}

// TODO: Implement for world, deferred world, etc.
