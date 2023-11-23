use bevy_ecs::{
    prelude::*,
    system::{Command, DriveSystem},
};
use bevy_reflect::prelude::*;
use bevy_utils::tracing;
use thiserror::Error;

use crate::{Asset, Assets, Handle, VisitAssetDependencies};

// TODO: Add docs
#[derive(TypePath)]
pub struct Callback<I = (), O = ()> {
    inner: Option<DriveSystem<I, O>>,
}

impl<I: 'static, O: 'static> Callback<I, O> {
    // TODO: Add docs
    pub fn from_system<M, S: IntoSystem<I, O, M>>(system: S) -> Self {
        Self {
            inner: Some(DriveSystem::new(Box::new(IntoSystem::into_system(system)))),
        }
    }
}

impl<I, O> VisitAssetDependencies for Callback<I, O> {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(crate::UntypedAssetId)) {
        // TODO: Would there be a way to get this info from the IntoSystem used to contruct the callback?
        // TODO: Should there be a way to pass this info through a different construct function?
    }
}

impl<I: TypePath, O: TypePath> Asset for Callback<I, O> {}

// TODO: Add docs
#[derive(Error)]
pub enum CallbackError<I: TypePath, O: TypePath> {
    // TODO: Add docs
    #[error("Callback {0:?} was not found")]
    HandleNotFound(Handle<Callback<I, O>>),
    // TODO: Add docs
    #[error("Callback {0:?} tried to run itself recursively")]
    Recursive(Handle<Callback<I, O>>),
    // TODO: Add docs
    #[error("Callback {0:?} removed itself")]
    RemovedSelf(Handle<Callback<I, O>>),
}

impl<I: TypePath, O: TypePath> std::fmt::Debug for CallbackError<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleNotFound(arg0) => f.debug_tuple("HandleNotFound").field(arg0).finish(),
            Self::Recursive(arg0) => f.debug_tuple("Recursive").field(arg0).finish(),
            Self::RemovedSelf(arg0) => f.debug_tuple("RemovedSelf").field(arg0).finish(),
        }
    }
}

// TODO: Add docs
pub trait RunCallbackWorld {
    // TODO: Add docs
    fn run_callback_with_input<In: TypePath + Send + 'static, Out: TypePath + 'static>(
        &mut self,
        handle: Handle<Callback<In, Out>>,
        input: In,
    ) -> Result<Out, CallbackError<In, Out>>;

    // TODO: Add docs
    fn run_callback<Out: TypePath + 'static>(
        &mut self,
        handle: Handle<Callback<(), Out>>,
    ) -> Result<Out, CallbackError<(), Out>> {
        self.run_callback_with_input(handle, ())
    }
}

impl RunCallbackWorld for World {
    fn run_callback_with_input<In: TypePath + Send + 'static, Out: TypePath + 'static>(
        &mut self,
        handle: Handle<Callback<In, Out>>,
        input: In,
    ) -> Result<Out, CallbackError<In, Out>> {
        let mut assets = self.resource_mut::<Assets<Callback<In, Out>>>();
        let mut callback = assets
            .get_mut(&handle)
            .ok_or_else(|| CallbackError::HandleNotFound(handle.clone()))?
            .inner
            .take()
            .ok_or_else(|| CallbackError::Recursive(handle.clone()))?;

        let result = callback.run_with_input(self, input);
        let mut assets = self.resource_mut::<Assets<Callback<In, Out>>>();
        assets
            .get_mut(&handle)
            .ok_or_else(|| CallbackError::RemovedSelf(handle))?
            .inner = Some(callback);

        Ok(result)
    }
}

// TODO: add docs
#[derive(Debug, Clone)]
pub struct RunCallbackWithInput<I: TypePath + 'static> {
    handle: Handle<Callback<I>>,
    input: I,
}

// TODO: add docs
pub type RunCallback = RunCallbackWithInput<()>;

impl RunCallback {
    // TODO: add docs
    pub fn new(handle: Handle<Callback>) -> Self {
        Self::new_with_input(handle, ())
    }
}

impl<I: TypePath + 'static> RunCallbackWithInput<I> {
    // TODO: add docs
    pub fn new_with_input(handle: Handle<Callback<I>>, input: I) -> Self {
        Self { handle, input }
    }
}

impl<I: TypePath + Send + 'static> Command for RunCallbackWithInput<I> {
    #[inline]
    fn apply(self, world: &mut World) {
        if let Err(error) = world.run_callback_with_input(self.handle, self.input) {
            tracing::error!("{error}");
        }
    }
}

// TODO: Add docs
pub trait RunCallbackCommands {
    // TODO: Add docs
    fn run_callback_with_input<I: TypePath + Send + 'static>(
        &mut self,
        handle: Handle<Callback<I>>,
        input: I,
    );

    // TODO: Add docs
    fn run_callback(&mut self, handle: Handle<Callback>) {
        self.run_callback_with_input(handle, ());
    }
}

impl<'w, 's> RunCallbackCommands for Commands<'w, 's> {
    fn run_callback_with_input<I: TypePath + Send + 'static>(
        &mut self,
        handle: Handle<Callback<I>>,
        input: I,
    ) {
        self.add(RunCallbackWithInput::new_with_input(handle, input));
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use bevy_ecs::prelude::*;

    #[derive(Resource, Default, PartialEq, Debug)]
    struct Counter(u8);

    #[test]
    fn change_detection() {
        #[derive(Resource, Default)]
        struct ChangeDetector;

        fn count_up_iff_changed(
            mut counter: ResMut<Counter>,
            change_detector: ResMut<ChangeDetector>,
        ) {
            if change_detector.is_changed() {
                counter.0 += 1;
            }
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_resource::<ChangeDetector>();
        app.init_resource::<Counter>();
        assert_eq!(*app.world.resource::<Counter>(), Counter(0));

        // Resources are changed when they are first added.
        let mut callbacks = app.world.resource_mut::<Assets<Callback>>();
        let handle = callbacks.add(Callback::from_system(count_up_iff_changed));
        app.world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));
        // Nothing changed
        app.world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));
        // Making a change
        app.world.resource_mut::<ChangeDetector>().set_changed();
        app.world
            .run_callback(handle)
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(2));
    }

    #[test]
    fn local_variables() {
        // The `Local` begins at the default value of 0
        fn doubling(mut last_counter: Local<Counter>, mut counter: ResMut<Counter>) {
            counter.0 += last_counter.0;
            last_counter.0 = counter.0;
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.insert_resource(Counter(1));
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));

        let mut callbacks = app.world.resource_mut::<Assets<Callback>>();
        let handle = callbacks.add(Callback::from_system(doubling));
        app.world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));
        app.world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(2));
        app.world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(4));
        app.world
            .run_callback(handle)
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(8));
    }

    #[test]
    fn input_values() {
        // Verify that a non-Copy, non-Clone type can be passed in.
        #[derive(TypePath)]
        struct NonCopy(u8);

        fn increment_sys(In(NonCopy(increment_by)): In<NonCopy>, mut counter: ResMut<Counter>) {
            counter.0 += increment_by;
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Callback<NonCopy>>();

        let mut callbacks = app.world.resource_mut::<Assets<Callback<NonCopy>>>();
        let handle = callbacks.add(Callback::from_system(increment_sys));

        // Insert the resource after registering the system.
        app.insert_resource(Counter(1));
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));

        app.world
            .run_callback_with_input(handle.clone(), NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(2));

        app.world
            .run_callback_with_input(handle.clone(), NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(3));

        app.world
            .run_callback_with_input(handle.clone(), NonCopy(20))
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(23));

        app.world
            .run_callback_with_input(handle, NonCopy(1))
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(24));
    }

    #[test]
    fn output_values() {
        // Verify that a non-Copy, non-Clone type can be returned.
        #[derive(TypePath, Eq, PartialEq, Debug)]
        struct NonCopy(u8);

        fn increment_sys(mut counter: ResMut<Counter>) -> NonCopy {
            counter.0 += 1;
            NonCopy(counter.0)
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Callback<(), NonCopy>>();

        let mut callbacks = app.world.resource_mut::<Assets<Callback<(), NonCopy>>>();
        let handle = callbacks.add(Callback::from_system(increment_sys));

        // Insert the resource after registering the system.
        app.insert_resource(Counter(1));
        assert_eq!(*app.world.resource::<Counter>(), Counter(1));

        let output = app
            .world
            .run_callback(handle.clone())
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(2));
        assert_eq!(output, NonCopy(2));

        let output = app
            .world
            .run_callback(handle)
            .expect("system runs successfully");
        assert_eq!(*app.world.resource::<Counter>(), Counter(3));
        assert_eq!(output, NonCopy(3));
    }

    #[test]
    fn nested_systems() {
        #[derive(Component)]
        struct Call(Handle<Callback>);

        fn nested(query: Query<&Call>, mut commands: Commands) {
            for call in query.iter() {
                commands.run_callback(call.0.clone());
            }
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.insert_resource(Counter(0));

        let mut callbacks = app.world.resource_mut::<Assets<Callback>>();

        let increment_two = callbacks.add(Callback::from_system(|mut counter: ResMut<Counter>| {
            counter.0 += 2;
        }));
        let increment_three =
            callbacks.add(Callback::from_system(|mut counter: ResMut<Counter>| {
                counter.0 += 3;
            }));
        let nested_handle = callbacks.add(Callback::from_system(nested));

        app.world.spawn(Call(increment_two));
        app.world.spawn(Call(increment_three));
        let _ = app.world.run_callback(nested_handle);
        assert_eq!(*app.world.resource::<Counter>(), Counter(5));
    }

    #[test]
    fn nested_systems_with_inputs() {
        #[derive(Component)]
        struct Call(Handle<Callback<u8>>, u8);

        fn nested(query: Query<&Call>, mut commands: Commands) {
            for callback in query.iter() {
                commands.run_callback_with_input(callback.0.clone(), callback.1);
            }
        }

        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Callback<u8>>();
        app.insert_resource(Counter(0));

        let mut callbacks = app.world.resource_mut::<Assets<Callback<u8>>>();

        let increment_by = callbacks.add(Callback::from_system(
            |In(amt): In<u8>, mut counter: ResMut<Counter>| {
                counter.0 += amt;
            },
        ));
        let mut callbacks = app.world.resource_mut::<Assets<Callback>>();
        let nested_id = callbacks.add(Callback::from_system(nested));

        app.world.spawn(Call(increment_by.clone(), 2));
        app.world.spawn(Call(increment_by, 3));
        let _ = app.world.run_callback(nested_id);
        assert_eq!(*app.world.resource::<Counter>(), Counter(5));
    }
}
