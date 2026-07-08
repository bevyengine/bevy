use core::{
    alloc::Layout,
    any::{type_name, Any, TypeId},
    mem::transmute,
    ptr::NonNull,
};
use std::{
    alloc::{alloc, dealloc},
    boxed::Box,
    vec::Vec,
};

use bevy_ecs::{
    component::{Component, Mutable},
    message::Message,
    observer::{IntoObserver, Observer},
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
    world::{FromWorld, World},
};

use crate::App;

pub struct ErasedResource(WideErased);

struct WideErased {
    layout: Layout,
    ptr: NonNull<()>,
    drop_fn: Box<dyn Fn(NonNull<()>)>,
    type_id: TypeId,
}

#[allow(unsafe_code)]
impl WideErased {
    pub fn new<T: Sized + Drop + 'static>(data: T) -> Option<Self> {
        let layout = Layout::for_value(&data);
        // SAFETY: we're allocating baybe. We initialize after the nonnull cast.
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr)?.cast();
        // SAFETY:
        unsafe { ptr.write(data) };

        Some(WideErased {
            layout,
            ptr: ptr.cast(),
            drop_fn: Box::new(move |ptr| {
                // SAFETY: These things cannot change, genuinely.
                let data: T = Self::nonnull_ptr_shuffle(ptr.cast(), layout);
                drop(data);
            }),
            type_id: TypeId::of::<T>(),
        })
    }

    pub fn try_reverse_erase<T: Sized + 'static>(self) -> Result<T, Self> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id {
            // SAFETY: we at least know if the data is the right shape.
            let data: NonNull<T> = self.ptr.cast();
            Ok(Self::nonnull_ptr_shuffle(data, layout))
        } else {
            Err(self)
        }
    }

    fn nonnull_ptr_shuffle<T>(ptr: NonNull<T>, layout: Layout) -> T {
        // SAFETY: we deallocate immediately after.
        let data_read = unsafe { ptr.read() };
        // SAFETY: the data is read to the stack already, we can free the ptr
        unsafe { dealloc(ptr.cast::<u8>().as_ptr(), layout) };
        data_read
    }
}

impl Drop for WideErased {
    fn drop(&mut self) {
        (self.drop_fn)(self.ptr);
    }
}

/// Plugin output, opaque to end user.
pub struct PluginOutput {
    working_plugin: PluginTypeId,
    // Hold onto the App for now. This should be moved in future.
    app: App,
    observers: Vec<Observer>,
    schedules: Schedules,
    resource_staging: Vec<(
        TypeId,
        ErasedResource,
        Box<dyn Fn(&mut World, ErasedResource)>,
    )>,
    dependencies: Vec<(PluginTypeId, Box<dyn Fn(&dyn DeclarativePlugin) -> bool>)>,
}

impl PluginOutput {
    /// Woah add systems and whatnot
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        // TODO
        self.schedules.add_systems(schedule, systems);
        self
    }

    pub fn add_observer<M>(&mut self, observer: impl IntoObserver<M>) -> &mut Self {
        self.observers.push(observer.into_observer());
        self
    }

    pub fn add_dependency_no_worries<P: DeclarativePlugin + Default>(&mut self) -> &mut Self {
        self.add_dependency::<P, _>(|_| true)
    }

    pub fn add_dependency<P: DeclarativePlugin + Default, F: Fn(&P) -> bool + 'static>(
        &mut self,
        evaluate_config: F,
    ) -> &mut Self {
        self.add_dependency_with_plugin_config(P::default(), evaluate_config);
        self
    }

    pub fn add_message<M: Message>(&mut self) -> &mut Self {
        self.app.main_mut().add_message::<M>();
        self
    }

    pub fn insert_resource<R: Resource + Drop>(&mut self, resource: R) -> &mut Self {
        self.resource_staging.push((
            resource.type_id(),
            ErasedResource(WideErased::new(resource).unwrap()),
            Box::new(|world, erased| match erased.0.try_reverse_erase::<R>() {
                Ok(resource) => world.insert_resource(resource),
                Err(_) => todo!(),
            }),
        ));
        self
    }

    /// Add a plugin dependency to the plugin output
    pub fn add_dependency_with_plugin_config<P: DeclarativePlugin, F: Fn(&P) -> bool + 'static>(
        &mut self,
        plugin: P,
        evaluate_config: F,
    ) -> &mut Self {
        let plugin_type_id = PluginTypeId(plugin.type_id());
        let evaluate_config = move |a: &dyn DeclarativePlugin| match <dyn Any>::downcast_ref::<P>(a)
        {
            Some(a) => evaluate_config(a),
            None => true,
        };
        self.dependencies
            .push((plugin_type_id, Box::new(evaluate_config)));
        self
    }

    pub fn add_dependency_with_plugin_config_no_worries<P: DeclarativePlugin>(
        &mut self,
        plugin: P,
    ) -> &mut Self {
        self.add_dependency_with_plugin_config::<P, _>(plugin, |_| true);
        self
    }
}

pub struct PluginTypeId(TypeId);

pub trait DeclarativePlugin: Any {
    fn build(&self, output: &mut PluginOutput);
}

/// The accumulated plugins
pub struct PluginPreGraph {
    nodes: Vec<(Box<dyn DeclarativePlugin>, PluginOutput)>,
}

pub struct PluginGraph {
    nodes: Vec<(PluginTypeId, Box<dyn DeclarativePlugin>)>,
    edges: Vec<(
        PluginTypeId,
        PluginTypeId,
        Box<dyn Fn(&dyn DeclarativePlugin) -> bool>,
    )>,
}
