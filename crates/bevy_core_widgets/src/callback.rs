use bevy_ecs::system::{Commands, IntoSystem, SystemId, SystemInput};
use bevy_ecs::template::{GetTemplate, Template};
use bevy_ecs::world::{DeferredWorld, World};
use bevy_reflect::Reflect;
use std::marker::PhantomData;

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
#[derive(Debug, Reflect)]
pub enum Callback<I: SystemInput = ()> {
    /// Invoke a one-shot system
    System(SystemId<I>),
    /// Ignore this notification
    Ignore,
}

impl<I: SystemInput> Copy for Callback<I> {}
impl<I: SystemInput> Clone for Callback<I> {
    fn clone(&self) -> Self {
        match self {
            Self::System(arg0) => Self::System(arg0.clone()),
            Self::Ignore => Self::Ignore,
        }
    }
}

impl<In: SystemInput + 'static> GetTemplate for Callback<In> {
    type Template = CallbackTemplate<In>;
}

#[derive(Default)]
pub enum CallbackTemplate<In: SystemInput = ()> {
    System(Box<dyn RegisterSystem<In>>),
    SystemId(SystemId<In>),
    #[default]
    Ignore,
}

impl<In: SystemInput + 'static> CallbackTemplate<In> {
    pub fn clone(&self) -> CallbackTemplate<In> {
        match self {
            CallbackTemplate::System(register_system) => {
                CallbackTemplate::System(register_system.box_clone())
            }
            CallbackTemplate::SystemId(system_id) => CallbackTemplate::SystemId(*system_id),
            CallbackTemplate::Ignore => CallbackTemplate::Ignore,
        }
    }
}

pub trait RegisterSystem<In: SystemInput>: Send + Sync + 'static {
    fn register_system(&mut self, world: &mut World) -> SystemId<In>;
    fn box_clone(&self) -> Box<dyn RegisterSystem<In>>;
}

pub struct IntoWrapper<I, In, Marker> {
    into_system: Option<I>,
    marker: PhantomData<fn() -> (In, Marker)>,
}

pub fn callback<
    I: IntoSystem<In, (), Marker> + Send + Sync + Clone + 'static,
    In: SystemInput + 'static,
    Marker: 'static,
>(
    system: I,
) -> CallbackTemplate<In> {
    CallbackTemplate::from(IntoWrapper {
        into_system: Some(system),
        marker: PhantomData,
    })
}

impl<
        I: IntoSystem<In, (), Marker> + Clone + Send + Sync + 'static,
        In: SystemInput + 'static,
        Marker: 'static,
    > RegisterSystem<In> for IntoWrapper<I, In, Marker>
{
    fn register_system(&mut self, world: &mut World) -> SystemId<In> {
        world.register_system(self.into_system.take().unwrap())
    }

    fn box_clone(&self) -> Box<dyn RegisterSystem<In>> {
        Box::new(IntoWrapper {
            into_system: self.into_system.clone(),
            marker: PhantomData,
        })
    }
}

impl<
        I: IntoSystem<In, (), Marker> + Clone + Send + Sync + 'static,
        In: SystemInput + 'static,
        Marker: 'static,
    > From<IntoWrapper<I, In, Marker>> for CallbackTemplate<In>
{
    fn from(value: IntoWrapper<I, In, Marker>) -> Self {
        CallbackTemplate::System(Box::new(value))
    }
}

impl<In: SystemInput + 'static> Template for CallbackTemplate<In> {
    type Output = Callback<In>;

    fn build(
        &mut self,
        entity: &mut bevy_ecs::world::EntityWorldMut,
    ) -> bevy_ecs::error::Result<Self::Output> {
        Ok(match self {
            CallbackTemplate::System(register) => {
                let id = entity.world_scope(move |world| register.register_system(world));
                *self = CallbackTemplate::SystemId(id);
                Callback::System(id)
            }
            CallbackTemplate::SystemId(id) => Callback::System(*id),
            CallbackTemplate::Ignore => Callback::Ignore,
        })
    }
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
