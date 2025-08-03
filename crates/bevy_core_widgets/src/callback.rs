use bevy_ecs::system::{Commands, SystemId, SystemInput};
use bevy_ecs::world::{DeferredWorld, World};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

/// A callback defines how we want to be notified when a widget changes state. Unlike an event
/// or observer, callbacks are intended for "point-to-point" communication that cuts across the
/// hierarchy of entities. Callbacks can be created in advance of the entity they are attached
/// to, and can be passed around as parameters.
///
/// Example:
/// ```
/// use bevy_app::App;
/// use bevy_core_widgets::{Callback, Notify};
/// use bevy_ecs::system::{Commands, IntoSystem};
///
/// let mut app = App::new();
///
/// // Register a one-shot system
/// fn my_callback_system() {
///     println!("Callback executed!");
/// }
///
/// let system_id = app.world_mut().register_system(my_callback_system);
///
/// // Wrap system in a callback
/// let callback = Callback::System(system_id);
///
/// // Later, when we want to execute the callback:
/// app.world_mut().commands().notify(&callback);
/// ```
#[derive(Default, Debug, Reflect)]
#[reflect(Default)]
pub enum Callback<I: SystemInput = ()> {
    /// Invoke a one-shot system
    System(SystemId<I>),
    /// Ignore this notification
    #[default]
    Ignore,
}

/// Trait used to invoke a [`Callback`], unifying the API across callers.
pub trait Notify {
    /// Invoke the callback with no arguments.
    fn notify(&mut self, callback: &Callback<()>);

    /// Invoke the callback with one argument.
    fn notify_with<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
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

    fn notify_with<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        match callback {
            Callback::System(system_id) => self.run_system_with(*system_id, input),
            Callback::Ignore => (),
        }
    }
}

impl Notify for World {
    fn notify(&mut self, callback: &Callback<()>) {
        match callback {
            Callback::System(system_id) => {
                let _ = self.run_system(*system_id);
            }
            Callback::Ignore => (),
        }
    }

    fn notify_with<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        match callback {
            Callback::System(system_id) => {
                let _ = self.run_system_with(*system_id, input);
            }
            Callback::Ignore => (),
        }
    }
}

impl Notify for DeferredWorld<'_> {
    fn notify(&mut self, callback: &Callback<()>) {
        match callback {
            Callback::System(system_id) => {
                self.commands().run_system(*system_id);
            }
            Callback::Ignore => (),
        }
    }

    fn notify_with<I>(&mut self, callback: &Callback<I>, input: I::Inner<'static>)
    where
        I: SystemInput<Inner<'static>: Send> + 'static,
    {
        match callback {
            Callback::System(system_id) => {
                self.commands().run_system_with(*system_id, input);
            }
            Callback::Ignore => (),
        }
    }
}
