use core::any::{type_name, Any, TypeId};
use std::{boxed::Box, vec::Vec};

use bevy_ecs::{
    message::Message, observer::{IntoObserver, Observer}, resource::Resource, schedule::{IntoScheduleConfigs, ScheduleLabel, Schedules}, system::ScheduleSystem, world::FromWorld,
};

use crate::App;

/// Plugin output, opaque to end user.
pub struct PluginOutput {
    working_plugin: PluginTypeId,
    // Hold onto the App for now. This should be moved in future.
    app: App,
    observers: Vec<Observer>,
    schedules: Schedules,
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

    pub fn add_dependency_no_worries<P: DeclarativePlugin + Default,>(
        &mut self,
    ) -> &mut Self {
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

    pub fn insert_resource<R: Resource>(&mut self, resource: R, ) -> &mut Self {
        self.app.main_mut().insert_resource(resource);
        self
    }

    /// Add a plugin dependency to the plugin output
    pub fn add_dependency_with_plugin_config<P: DeclarativePlugin, F: Fn(&P) -> bool + 'static>(
        &mut self,
        plugin: P,
        evaluate_config: F,
    ) -> &mut Self {
        let plugin_type_id = PluginTypeId(plugin.type_id());
        let evaluate_config = move |a: &dyn DeclarativePlugin| {
            match <dyn Any>::downcast_ref::<P>(a) {
                Some(a) => evaluate_config(a),
                None => true,
            }
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
    nodes: Vec<Box<dyn DeclarativePlugin>>,
}

pub struct PluginGraph {
    nodes: Vec<(PluginTypeId, Box<dyn DeclarativePlugin>)>,
    edges: Vec<(
        PluginTypeId,
        PluginTypeId,
        Box<dyn Fn(&dyn DeclarativePlugin) -> bool>,
    )>,
}

pub struct DPlug;

impl DeclarativePlugin for DPlug {
    fn build(&self, output: &mut PluginOutput) {}
}

fn lol(a: Box<dyn DeclarativePlugin>) {
    match <dyn Any>::downcast_ref::<DPlug>(&a) {
        Some(s) => todo!(),
        None => todo!(),
    }
    ()
}
