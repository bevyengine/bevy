use bevy_ecs::{
    message::Message,
    observer::{IntoObserver, Observer},
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules},
    system::ScheduleSystem,
};
use bevy_platform::collections::{HashMap, HashSet};
use core::{
    any::{Any, TypeId},
    hash::Hash,
};
use std::{boxed::Box, vec::Vec};

use crate::{approval::Approval, erased_resource::StagedResource, App, DeclarativePlugin};

/// A list of plugin dependencies for a particular [`PluginOutput`].
pub struct PluginDependencies(pub(crate) Vec<PluginDependency>);

/// Plugin output, opaque to end user.
///
/// This is designed to be a plugin data structure that end users don't need to
/// think about in terms of what it's "made of." Just something that stuff can be
/// added to.
///
/// TODO: docs that are user-facing, not reviewer-facing.
pub struct PluginOutput<D = PluginDependencies> {
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
            dependencies: PluginDependencies(Vec::new()),
            plugin_approval: HashMap::new(),
        }
    }

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
            .0
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

pub(crate) struct PluginDependency {
    pub(crate) type_id: PluginTypeId,
    /// An optional pairing of a plugin's data (as initialized by the plugin depending on it) and an erased function that builds the plugin output for that dependency.
    pub(crate) data: Option<(
        Box<dyn DeclarativePlugin>,
        Box<dyn Fn(&dyn DeclarativePlugin) -> Option<PluginOutput>>,
    )>,
}

impl PluginDependency {
    pub(crate) fn new_with_config<P: DeclarativePlugin + 'static>(plugin: P) -> Self {
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub(crate) struct PluginTypeId(TypeId);

pub(crate) struct MessageRegistration {
    // A function that actually registers the message type with the App/World.
    registration_func: Box<dyn Fn(&mut App)>,
}

impl MessageRegistration {
    pub(crate) fn new<T: Message>() -> Self {
        Self {
            registration_func: Box::new(|app| {
                app.add_message::<T>();
            }),
        }
    }
}

pub(crate) struct MergeableSchedule {
    schedules: Schedules,
}

impl MergeableSchedule {
    pub(crate) fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        // TODO: non-schedule data structure?
        self.schedules.add_systems(schedule, systems);
    }
}
