use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::system::{Commands, EntityCommands, IntoSystem, SystemId, SystemInput};
use bevy_ecs::world::{DeferredWorld, EntityWorldMut, World};

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
#[derive(Default, Debug)]
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

/// A component that hangs on to a registered one-shot system, and unregisters it when the
/// component is despawned.
#[derive(Component)]
#[component(on_remove = on_despawn_callback_owner::<I>, storage = "SparseSet")]
pub struct OwnedCallbackSystem<I: SystemInput + Send>(SystemId<I, ()>);

fn on_despawn_callback_owner<I: SystemInput + Send + 'static>(
    mut world: DeferredWorld,
    context: HookContext,
) {
    let system_id = world
        .entity(context.entity)
        .get::<OwnedCallbackSystem<I>>()
        .unwrap()
        .0;
    world.commands().unregister_system(system_id);
}

/// Methods for registering scoped callbacks.
pub trait RegisterOwnedCallback {
    /// Registers a scoped one-shot system, with no input, that will be removed when the parent
    /// entity is despawned.
    fn register_owned_callback<M, I: IntoSystem<(), (), M> + 'static>(
        &mut self,
        callback: I,
    ) -> Callback;

    /// Registers a scoped one-shot systemm, with input, that will be removed when the
    /// parent entity is despawned.
    fn register_owned_callback_with<
        M,
        A: SystemInput + Send + 'static,
        I: IntoSystem<A, (), M> + 'static,
    >(
        &mut self,
        callback: I,
    ) -> Callback<A>;
}

impl RegisterOwnedCallback for EntityCommands<'_> {
    fn register_owned_callback<M, I: IntoSystem<(), (), M> + 'static>(
        &mut self,
        callback: I,
    ) -> Callback {
        let system_id = self.commands().register_system(callback);
        let owner = self.id();
        self.commands()
            .spawn((OwnedCallbackSystem(system_id), crate::owner::OwnedBy(owner)));
        Callback::System(system_id)
    }

    fn register_owned_callback_with<
        M,
        A: SystemInput + Send + 'static,
        I: IntoSystem<A, (), M> + 'static,
    >(
        &mut self,
        callback: I,
    ) -> Callback<A> {
        let owner = self.id();
        let system_id = self.commands().register_system(callback);
        self.commands()
            .spawn((OwnedCallbackSystem(system_id), crate::owner::OwnedBy(owner)));
        Callback::System(system_id)
    }
}

impl RegisterOwnedCallback for EntityWorldMut<'_> {
    fn register_owned_callback<M, I: IntoSystem<(), (), M> + 'static>(
        &mut self,
        callback: I,
    ) -> Callback {
        let owner = self.id();
        let system_id = self.world_scope(|world| world.register_system(callback));
        self.world_scope(|world| {
            world.spawn((OwnedCallbackSystem(system_id), crate::owner::OwnedBy(owner)));
        });
        Callback::System(system_id)
    }

    fn register_owned_callback_with<
        M,
        A: SystemInput + Send + 'static,
        I: IntoSystem<A, (), M> + 'static,
    >(
        &mut self,
        callback: I,
    ) -> Callback<A> {
        let owner = self.id();
        let system_id = self.world_scope(|world| world.register_system(callback));
        self.world_scope(|world| {
            world.spawn((OwnedCallbackSystem(system_id), crate::owner::OwnedBy(owner)));
        });
        Callback::System(system_id)
    }
}
