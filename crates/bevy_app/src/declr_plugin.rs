use bevy_ecs::{
    message::Message,
    observer::{IntoObserver, Observer},
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
    world::World,
};
use bevy_platform::collections::HashMap;
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    ptr::NonNull,
};
use std::{
    alloc::{alloc, dealloc},
    boxed::Box,
    vec::Vec,
};

use crate::App;

/// A function that, given a type (and a context), returns a boolean.
///
/// Defined and used here in the context of solving the dependency graph by
/// "asking" each dependency if a given resource or plugin config is appropriate
/// for its needs.
///
/// Most plugins shouldn't be picky, all they require is the _presence_ of a
/// resource or other plugin. But some might have tighter, runtime-known constraints.
///
/// Approval functions are expected to, mostly, return true.
pub struct Approval<T: ?Sized, Ctx = ()> {
    approval_fn: Box<dyn Fn(&T, &Ctx) -> bool>,
}

impl<T> Approval<T, ()> {
    /// Creates a new approval function which does not care about context.
    pub(crate) fn new(approval: impl Fn(&T) -> bool + 'static) -> Self {
        Self {
            approval_fn: Box::new(move |input, _ctx| approval(input)),
        }
    }
    /// "Asks" if the input is good enough.
    pub(crate) fn approves(&self, input: &T) -> bool {
        (self.approval_fn)(input, &())
    }
}

impl<T, Ctx> Approval<T, Ctx> {
    /// Creates a new approval function that does care about context.
    pub(crate) fn new_with_context(approval: impl Fn(&T, &Ctx) -> bool + 'static) -> Self {
        Self {
            approval_fn: Box::new(approval),
        }
    }

    /// "Asks" if the input and context is good enough
    pub(crate) fn approves_with_context(&self, input: &T, ctx: &Ctx) -> bool {
        (self.approval_fn)(input, ctx)
    }
}

/// A type erased [`Resource`], implemented using a [`MetadataPtr`]. This is
/// necessary due to [`Resource`] not being dyn compatible.
pub struct ErasedResource(MetadataPtr);

/// Fully type erased pointer that owns the data, knows the layout, knows the
/// [`TypeId`], and holds onto a copy of the drop implementation.
struct MetadataPtr {
    layout: Layout,
    ptr: NonNull<()>,
    drop_fn: Box<dyn Fn(NonNull<()>, Layout)>,
    type_id: TypeId,
}

#[allow(unsafe_code)]
impl MetadataPtr {
    pub fn new<T: Sized + 'static>(data: T) -> Option<Self> {
        let layout = Layout::for_value(&data);
        // SAFETY: Initialization happens in the next unsafe block, there's no
        // branching before then.
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr)?.cast();
        // SAFETY: This uses a Layout derived from T
        unsafe { ptr.write(data) };

        Some(MetadataPtr {
            layout,
            ptr: ptr.cast(),
            drop_fn: Box::new(|ptr, layout| {
                // SAFETY: These things cannot change, genuinely.
                let data: T = Self::move_then_deallocate(ptr.cast(), layout);
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
            Ok(Self::move_then_deallocate(data, layout))
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

    fn move_then_deallocate<T>(ptr: NonNull<T>, layout: Layout) -> T {
        // SAFETY: we deallocate immediately after.
        let data_read = unsafe { ptr.read() };
        // SAFETY: the data is read to the stack already, we can free the ptr
        unsafe { dealloc(ptr.cast::<u8>().as_ptr(), layout) };
        data_read
    }

    pub fn inner_type_id(&self) -> TypeId {
        self.type_id
    }
}

impl Drop for MetadataPtr {
    fn drop(&mut self) {
        (self.drop_fn)(self.ptr, self.layout);
    }
}

/// Data structure to get around the fact that Resource is not dyn compatible.
pub struct StagedResource {
    /// The type-erased resource.
    pub(crate) erased_resource: ErasedResource,
    /// Function that un-erases the resource and adds it to a given world.
    pub(crate) unerase_and_insert: Box<dyn Fn(&mut World, ErasedResource)>,
    /// Function that un-erases the resource and check if it meets the plugin's requirements.
    pub(crate) approval_from_plugin: Approval<ErasedResource>,
    /// If the staged resource is a default, we care about it less than any given
    pub(crate) is_default: bool,
}

impl StagedResource {
    fn new<R: Resource>(
        resource: R,
        is_default: bool,
        check_ok: impl Fn(&R) -> bool + 'static,
    ) -> Option<Self> {
        Some(StagedResource {
            erased_resource: ErasedResource(MetadataPtr::new(resource)?),
            unerase_and_insert: Box::new(|world, erased| match erased.0.try_reverse_erase::<R>() {
                Ok(resource) => world.insert_resource(resource),
                Err(_) => {}
            }),
            approval_from_plugin: Approval::new(move |erased: &ErasedResource| {
                erased
                    .0
                    .peek_reverse_erased::<R, _>(|data| check_ok(data))
                    // If the types didn't match, we should never veto (it's not this type's problem).
                    .unwrap_or(true)
            }),
            is_default,
        })
    }
}

pub struct MessageRegistration {
    // A function that actually registers the message type with the App/World.
    registration_func: Box<dyn Fn(&mut App)>,
}

impl MessageRegistration {
    pub fn new<T: Message>() -> Self {
        Self {
            registration_func: Box::new(|app| {
                app.add_message::<T>();
            }),
        }
    }
}

pub struct MergeableSchedule {
    schedules: Schedules,
}

impl MergeableSchedule {
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        // TODO: non-schedule data structure?
        self.schedules.add_systems(schedule, systems);
    }
}

pub struct PluginDependency {
    type_id: TypeId,
    /// An optional pairing of a plugin's data (as initialized by the plugin depending on it) and an erased function that builds the plugin output for that dependency.
    data: Option<(
        Box<dyn DeclarativePlugin>,
        Box<dyn Fn(&dyn DeclarativePlugin) -> Option<PluginOutput>>,
    )>,
}

impl PluginDependency {
    pub fn new_with_config<P: DeclarativePlugin + 'static>(plugin: P) -> Self {
        let data: Option<(
            Box<dyn DeclarativePlugin>,
            Box<dyn Fn(&dyn DeclarativePlugin) -> Option<PluginOutput>>,
        )> = Some((
            Box::new(plugin),
            Box::new(|plugin: &dyn DeclarativePlugin| {
                <dyn Any>::downcast_ref::<P>(plugin).map(|plugin| {
                    let mut output = PluginOutput::new::<P>();
                    plugin.build(&mut output);
                    output
                })
            }),
        ));
        PluginDependency {
            type_id: TypeId::of::<P>(),
            data,
        }
    }

    pub fn new<P: DeclarativePlugin + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<P>(),
            data: None,
        }
    }
}

/// Plugin output, opaque to end user.
///
/// This is designed to be a plugin data structure that end users don't need to
/// think about in terms of what it's "made of."
pub struct PluginOutput {
    /// Plugin type ID (used to build edges later)
    pub(crate) working_plugin: PluginTypeId,
    /// Observers registered by this plugin.
    pub(crate) observers: Vec<Observer>,
    /// The schedule graph for this plugin (to be merged with others later)
    // TODO: Either roll our own
    pub(crate) schedules: MergeableSchedule,
    /// Message storage
    pub(crate) messages: HashMap<TypeId, MessageRegistration>,
    /// Resource storage
    pub(crate) resource_staging: Vec<StagedResource>,
    /// Plugin dependencies
    pub(crate) dependencies: Vec<PluginDependency>,
}

impl PluginOutput {
    // Create a plugin output structure for a given plugin type.
    pub(crate) fn new<P: 'static>() -> Self {
        Self {
            working_plugin: PluginTypeId(TypeId::of::<P>()),
            observers: Vec::new(),
            schedules: MergeableSchedule {
                schedules: Schedules::new(),
            },
            messages: HashMap::new(),
            resource_staging: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    ///
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
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

    pub fn add_message<M: Message + 'static>(&mut self) -> &mut Self {
        self.messages
            .insert(TypeId::of::<M>(), MessageRegistration::new::<M>());
        self
    }

    pub fn require_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.require_resource_with_approval(|_: &R| true)
    }

    pub fn require_exact_resource<R: Resource + Clone + PartialEq>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        let cloned = resource.clone();
        self.require_resource_with_value_and_approval(resource, move |resource: &R| {
            *resource == cloned
        })
    }

    pub fn require_resource_with_approval<R: Resource + Default>(
        &mut self,
        approval: impl Fn(&R) -> bool + 'static,
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

    pub fn require_resource_with_value_and_approval<R: Resource>(
        &mut self,
        resource: R,
        approval: impl Fn(&R) -> bool + 'static,
    ) -> &mut Self {
        let Some(resource) = StagedResource::new(resource, false, approval) else {
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
        self.dependencies
            .push(PluginDependency::new_with_config(plugin));
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

/// The accumulated plugins before being moved into a graph.
pub struct PluginList {
    nodes: Vec<(Box<dyn DeclarativePlugin>, PluginOutput)>,
}

pub struct PluginGraph {
    nodes: Vec<(PluginTypeId, Box<dyn DeclarativePlugin>)>,
    edges: Vec<(PluginTypeId, PluginTypeId, Approval<dyn DeclarativePlugin>)>,
}

/// The final order for things to be registered in.
pub struct OrderedPluginItems(Vec<DeclrItem>);

/// Items that can be added to a world.
pub enum DeclrItem {
    Message(MessageRegistration),
    // etc.
}
