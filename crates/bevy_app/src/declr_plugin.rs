use bevy_ecs::{
    message::Message,
    observer::{IntoObserver, Observer},
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
    world::World,
};
use bevy_platform::collections::{HashMap, HashSet};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    hash::Hash,
    ptr::NonNull,
};
use std::{
    alloc::{alloc, dealloc},
    boxed::Box,
    collections::VecDeque,
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
/// Approval functions are expected to, mostly, return true and neither contain
/// nor take take advantage of mutable state. Memoization rights reserved.
#[derive(Default)]
pub(crate) struct Approval<T: ?Sized, Ctx = ()> {
    approval_fn: Option<Box<dyn Fn(&T, &Ctx) -> bool>>,
}

impl<T> Approval<T, ()> {
    /// Creates a new approval function which does not care about context.
    pub(crate) fn new(approval: impl Fn(&T) -> bool + 'static) -> Self {
        Self {
            approval_fn: Some(Box::new(move |input, _ctx| approval(input))),
        }
    }
    /// "Asks" if the input is good enough.
    pub(crate) fn approves(&self, input: &T) -> bool {
        self.approval_fn
            .as_ref()
            .map(|f| f(input, &()))
            .unwrap_or(true)
    }
}

impl<T, Ctx> Approval<T, Ctx> {
    /// Create an approval function that will always return true.
    pub(crate) fn always_approve() -> Self {
        Self { approval_fn: None }
    }

    /// Creates a new approval function that does care about context.
    pub(crate) fn new_with_context(approval: impl Fn(&T, &Ctx) -> bool + 'static) -> Self {
        Self {
            approval_fn: Some(Box::new(approval)),
        }
    }

    /// "Asks" if the input and context is good enough
    pub(crate) fn approves_with_context(&self, input: &T, ctx: &Ctx) -> bool {
        self.approval_fn
            .as_ref()
            .map(|f| f(input, ctx))
            .unwrap_or(true)
    }
}

impl<T, F: Fn(&T) -> bool + 'static> From<F> for Approval<T, ()> {
    fn from(value: F) -> Self {
        Self::new(value)
    }
}

impl<T, Ctx> From<()> for Approval<T, Ctx> {
    fn from(value: ()) -> Self {
        Self::always_approve()
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
    already_dropped: bool,
}

#[allow(unsafe_code)]
impl MetadataPtr {
    pub fn new<T: Sized + 'static>(data: T) -> Option<Self> {
        let layout = Layout::for_value(&data);
        // SAFETY: Initialization happens in the next unsafe block, there's no
        // branching other than null pointer checking before then. Null pointers
        // cannot be deallocated.
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr)?.cast();
        // SAFETY: This uses a Layout derived from T
        unsafe { ptr.write(data) };

        Some(MetadataPtr {
            layout,
            ptr: ptr.cast(),
            drop_fn: Box::new(|ptr, layout| {
                // SAFETY: this function is only ever passed the original layout.
                let data: T = unsafe { Self::move_then_deallocate(ptr.cast(), layout) };
                drop(data);
            }),
            type_id: TypeId::of::<T>(),
            already_dropped: false,
        })
    }

    pub fn try_reverse_erase<T: Sized + 'static>(mut self) -> Result<T, Self> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id && !self.already_dropped {
            // SAFETY: we at least know if the data is the right shape and the type IDs are the same.
            let data: NonNull<T> = self.ptr.cast();
            // SAFETY: We are passing the original layout this type was constructed with.
            if self.layout.size() != 0 {
                self.already_dropped = true;
                Ok(unsafe { Self::move_then_deallocate(data, self.layout) })
            } else {
                Err(self)
            }
        } else {
            Err(self)
        }
    }

    pub fn visit<T: Sized + 'static, Y>(&self, peek: impl for<'b> Fn(&'b T) -> Y) -> Option<Y> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id && !self.already_dropped {
            Some(peek(unsafe { self.ptr.cast().as_ref() }))
        } else {
            None
        }
    }

    /// SAFETY: The layout passed must be the same as what `ptr` was allocated with.
    unsafe fn move_then_deallocate<T>(ptr: NonNull<T>, layout: Layout) -> T {
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
        if !self.already_dropped {
            (self.drop_fn)(self.ptr, self.layout);
        }
    }
}

#[cfg(test)]
mod metadata_ptr_test {
    use crate::declr_plugin::MetadataPtr;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut v: Vec<u8> = Vec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        let erased = MetadataPtr::new(v.clone()).unwrap();
        let visit_res = erased.visit::<Vec<u8>, _>(|v| (v.len(), v.iter().fold(0, |a, b| a + b)));
        assert_eq!(Some((4, 10)), visit_res);
        let visit_res = erased.visit::<Vec<u8>, _>(|v| (v.len(), v.iter().fold(0, |a, b| a + b)));
        assert_eq!(Some((4, 10)), visit_res);
        let visit_none = erased.visit::<Vec<u16>, _>(|v| v.len());
        assert_eq!(None, visit_none);
        let visit_take_ref = erased.visit::<Vec<u8>, _>(|v| v.len());
        let un_erased = erased.try_reverse_erase::<Vec<u8>>();
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
                    .visit::<R, _>(|data| check_ok(data))
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
    type_id: PluginTypeId,
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
            type_id: PluginTypeId(TypeId::of::<P>()),
            data,
        }
    }

    pub fn new<P: DeclarativePlugin + 'static>() -> Self {
        Self {
            type_id: PluginTypeId(TypeId::of::<P>()),
            data: None,
        }
    }
}

/// Plugin output, opaque to end user.
///
/// This is designed to be a plugin data structure that end users don't need to
/// think about in terms of what it's "made of." Just something that stuff can be
/// added to.
///
/// TODO: docs that are user-facing, not reviewer-facing.
pub struct PluginOutput<D = Vec<PluginDependency>> {
    /// The plugin was added to an app, or part of a declarative bundle, rather
    /// than being inserted as a dependency.
    pub(crate) is_entry_point: bool,
    /// Is the plugin type zero-sized (most are). If a plugin is zero-sized we
    /// can make the assumption that all calls to [`DeclarativePlugin::build`]
    /// for that type give an identical [`PluginOutput`], as there is no
    /// configuration that can be done.
    pub(crate) is_zero_sized_optimizable: bool,
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
    pub(crate) plugin_approval: HashMap<PluginTypeId, Approval<Box<dyn DeclarativePlugin>>>,
    /// Plugin dependencies, represented with a generic so we can erase them in
    /// plugin graph resolution.
    pub(crate) dependencies: D,
}

impl<D> PluginOutput<D> {
    pub(crate) fn extract_dependencies(self) -> (PluginOutput<()>, D) {
        let Self {
            is_entry_point,
            is_zero_sized_optimizable,
            working_plugin,
            observers,
            schedules,
            messages,
            resource_staging,
            dependencies,
            plugin_approval,
        } = self;
        (
            PluginOutput {
                is_entry_point,
                is_zero_sized_optimizable,
                working_plugin,
                observers,
                schedules,
                messages,
                resource_staging,
                plugin_approval,
                dependencies: (),
            },
            dependencies,
        )
    }
}

impl PluginOutput {
    /// Create a plugin output structure for a given plugin type.
    pub(crate) fn new<P: 'static>() -> Self {
        Self {
            // This is set by App bookkeeping.
            is_entry_point: false,
            is_zero_sized_optimizable: size_of::<P>() == 0,
            working_plugin: PluginTypeId(TypeId::of::<P>()),
            observers: Vec::new(),
            schedules: MergeableSchedule {
                schedules: Schedules::new(),
            },
            messages: HashMap::new(),
            resource_staging: Vec::new(),
            dependencies: Vec::new(),
            plugin_approval: HashMap::new(),
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

    pub fn add_dependency<P: DeclarativePlugin + Default>(&mut self) -> &mut Self {
        self.add_dependency_with_approval::<P, _>(|_| true)
    }

    pub fn add_dependency_with_approval<
        P: DeclarativePlugin + Default,
        F: Fn(&P) -> bool + 'static,
    >(
        &mut self,
        approval: F,
    ) -> &mut Self {
        self.add_dependency_with_plugin_config_and_approval(P::default(), approval);
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
    pub fn add_dependency_with_plugin_config_and_approval<
        P: DeclarativePlugin,
        F: for<'a> Fn(&'a P) -> bool + 'static,
    >(
        &mut self,
        plugin: P,
        approval: F,
    ) -> &mut Self {
        self.dependencies
            .push(PluginDependency::new_with_config(plugin));
        self.add_dependency_approval::<P, _>(approval);
        self
    }

    pub fn add_dependency_with_plugin_config<P: DeclarativePlugin>(
        &mut self,
        plugin: P,
    ) -> &mut Self {
        self.add_dependency_with_plugin_config_and_approval::<P, _>(plugin, |_| true);
        self
    }

    fn add_dependency_approval<
        P: DeclarativePlugin + 'static,
        F: for<'a> Fn(&'a P) -> bool + 'static,
    >(
        &mut self,
        approval: F,
    ) {
        // We always approve zero-sized types. There is no config, and `|_| false` is considered a misbehave.
        let is_zst = size_of::<P>() == 0;
        let plugin_type_id = PluginTypeId(TypeId::of::<P>());
        if is_zst {
            self.plugin_approval
                .insert(plugin_type_id, Approval::always_approve());
        } else {
            self.plugin_approval.insert(
                plugin_type_id,
                Approval::new(move |dyn_plugin| {
                    let Some(plugin) = <dyn Any>::downcast_ref::<P>(dyn_plugin) else {
                        return false;
                    };
                    approval(plugin)
                }),
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub(crate) struct PluginTypeId(TypeId);

pub trait DeclarativePlugin: Any {
    fn build(&self, output: &mut PluginOutput);

    /// When this is a zero-sized type, it will give the same [`PluginOutput`]
    /// every time [`DeclarativePlugin`] is called.
    fn zero_sized_instances_are_identical(&self) -> bool {
        true
    }
}

/// A list of "entry point" plugins and their outputs. This gets expanded into a graph.
pub(crate) struct PluginList {
    nodes: Vec<(Box<dyn DeclarativePlugin>, PluginOutput)>,
}

impl PluginList {
    /// Expand the list of entry point plugins into a full graph. Ignores recurring ZSTs.
    pub(crate) fn expand(mut self) -> Result<PluginRegistrationGraph, ()> {
        let mut zst_already_expanded: HashMap<PluginTypeId, RegistrationId> = HashMap::new();
        let mut graph = PluginRegistrationGraph::new();
        for (_, output) in &mut self.nodes {
            // Mark entry points.
            output.is_entry_point = true;
        }
        let mut dependency_stack = VecDeque::new();
        for (item, output) in self.nodes {
            if output.is_zero_sized_optimizable
                && !zst_already_expanded.contains_key(&output.working_plugin)
            {
                let type_id = output.working_plugin;
                let (reg_id, dependencies) = graph.insert_node(output.working_plugin, item, output);
                dependency_stack.extend(dependencies.into_iter().map(|d| (reg_id, d)));
                zst_already_expanded.insert(type_id, reg_id);
            } else if !output.is_zero_sized_optimizable {
            }
        }
        // TODO: detect cycles in expansion + stop adding when "expanded enough" + solved.
        // mean moving this logic to the PluginRegistrationGraph building side.
        while let Some((from, dependency)) = dependency_stack.pop_front() {
            if !zst_already_expanded.contains_key(&dependency.type_id)
                && let Some((dyn_plugin, output_fn)) = dependency.data
            {
                let Some(output) = output_fn(dyn_plugin.as_ref()) else {
                    continue;
                };
                let plugin_id = output.working_plugin;
                let can_zst_optimize = output.is_zero_sized_optimizable;
                let (reg_id, dependencies) = graph.insert_node(plugin_id, dyn_plugin, output);
                graph.insert_edge(from, reg_id);
                dependency_stack.extend(dependencies.into_iter().map(|d| (reg_id, d)));
                if can_zst_optimize {
                    zst_already_expanded.insert(plugin_id, reg_id);
                }
            } else if dependency.data.is_none() {
                // TODO:
            }
        }
        Ok(graph)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Clone, Copy)]
pub(crate) struct RegistrationId(usize);

pub(crate) struct PluginRegistrationGraph {
    registration_counter: usize,
    nodes:
        HashMap<PluginTypeId, Vec<(RegistrationId, Box<dyn DeclarativePlugin>, PluginOutput<()>)>>,
    registration_type_association: HashMap<RegistrationId, PluginTypeId>,
    dependency_edges: HashMap<RegistrationId, Vec<RegistrationId>>,
}

impl PluginRegistrationGraph {
    fn new_id(&mut self) -> RegistrationId {
        let id = RegistrationId(self.registration_counter);
        self.registration_counter += 1;
        id
    }

    pub(crate) fn new() -> Self {
        Self {
            registration_counter: 0,
            nodes: HashMap::new(),
            dependency_edges: HashMap::new(),
            registration_type_association: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn insert_node<D>(
        &mut self,
        id: PluginTypeId,
        plugin_data: Box<dyn DeclarativePlugin>,
        output: PluginOutput<D>,
    ) -> (RegistrationId, D) {
        let registration_id = self.new_id();
        let (erased_output, dependencies) = output.extract_dependencies();
        self.nodes
            .entry(id)
            .or_default()
            .push((registration_id, plugin_data, erased_output));
        self.registration_type_association
            .insert(registration_id, id);
        (registration_id, dependencies)
    }

    pub(crate) fn insert_edge(&mut self, from: RegistrationId, to: RegistrationId) {
        self.dependency_edges.entry(from).or_default().push(to);
    }
}

/// The final order for things to be registered in.
pub struct OrderedPluginItems(Vec<DeclrItem>);

pub struct ItemsGraph {}

/// Items that can be added to a world.
pub enum DeclrItem {
    Message(MessageRegistration),
    // etc.
}
