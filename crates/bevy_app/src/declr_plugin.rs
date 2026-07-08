use core::{
    alloc::Layout,
    any::{type_name, Any, TypeId},
    mem::{transmute, ManuallyDrop},
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
    pub fn new<T: Sized + 'static>(data: T) -> Option<Self> {
        let layout = Layout::for_value(&data);
        // SAFETY: we're allocating baybe. We initialize after the nonnull cast.
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr)?.cast();
        // SAFETY: initializing lol
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
            // SAFETY: we at least know if the data is the right shape and the type IDs are the same
            let data: NonNull<T> = self.ptr.cast();
            Ok(Self::nonnull_ptr_shuffle(data, layout))
        } else {
            Err(self)
        }
    }

    pub fn peek_reverse_erased<'a, T: Sized + 'static, Y>(
        &'a self,
        peek: impl Fn(&T) -> Y,
    ) -> Option<Y> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id {
            Some(peek(unsafe { self.ptr.cast().as_ref() }))
        } else {
            None
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

pub struct StagedResource {
    type_id: TypeId,
    erased_resource: ErasedResource,
    unerase_and_insert: Box<dyn Fn(&mut World, ErasedResource)>,
    /// Function that un-erases the resource and check if it meets the plugin's requirements.
    check_ok: Box<dyn Fn(&ErasedResource) -> bool>,
    /// If the staged resource is a default, we care about it less than any given
    is_default: bool,
}

impl StagedResource {
    fn new<R: Resource>(
        resource: R,
        is_default: bool,
        check_ok: impl Fn(&R) -> bool + 'static,
    ) -> Option<Self> {
        Some(StagedResource {
            type_id: resource.type_id(),
            erased_resource: ErasedResource(WideErased::new(resource)?),
            unerase_and_insert: Box::new(|world, erased| match erased.0.try_reverse_erase::<R>() {
                Ok(resource) => world.insert_resource(resource),
                Err(_) => {}
            }),
            check_ok: Box::new(move |erased| {
                erased
                    .0
                    .peek_reverse_erased::<R, _>(|data| check_ok(data))
                    .unwrap_or(true)
            }),
            is_default,
        })
    }
}

/// Plugin output, opaque to end user.
pub struct PluginOutput {
    working_plugin: PluginTypeId,
    // Hold onto the App for now. This should be moved in future.
    app: App,
    observers: Vec<Observer>,
    schedules: Schedules,
    resource_staging: Vec<StagedResource>,
    dependencies: Vec<(PluginTypeId, Box<dyn Fn(&dyn DeclarativePlugin) -> bool>)>,
}

impl PluginOutput {
    /// Woah add systems and whatnot
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        // TODO: non-schedule data structure?
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

    pub fn require_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.require_resource_with_clash(|_: &R| true)
    }

    pub fn require_resource_with_clash<R: Resource + Default>(
        &mut self,
        clash: impl Fn(&R) -> bool + 'static,
    ) -> &mut Self {
        let Some(resource) = StagedResource::new(R::default(), true, |_| true) else {
            return self;
        };
        self.resource_staging.push(resource);
        self
    }

    pub fn require_resource_with_value<R: Resource>(&mut self, resource: R) -> &mut Self {
        let Some(resource) = StagedResource::new(resource, false, |_| true) else {
            return self;
        };
        self.resource_staging.push(resource);
        self
    }

    pub fn require_resource_with_value_and_clash<R: Resource>(
        &mut self,
        resource: R,
        clash: impl Fn(&R) -> bool + 'static,
    ) -> &mut Self {
        let Some(resource) = StagedResource::new(resource, false, clash) else {
            return self;
        };
        self.resource_staging.push(resource);
        self
    }

    #[deprecated]
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        let Some(resource) = StagedResource::new(resource, false, |_| true) else {
            return self;
        };
        self.resource_staging.push(resource);
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
