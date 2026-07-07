use core::any::{type_name, Any, TypeId};
use std::{boxed::Box, vec::Vec};

use bevy_ecs::{
    observer::IntoObserver,
    schedule::{IntoScheduleConfigs, ScheduleLabel},
    system::ScheduleSystem,
};

use crate::App;

/// Plugin output, opaque to end user.
pub struct PluginOutput {
    working_plugin: PluginTypeId,
    // Hold onto the App for now. This should be moved in future.
    app: App,
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
        self.app.main_mut().add_systems(schedule, systems);
        self
    }

    pub fn add_observer<M>(&mut self, observer: impl IntoObserver<M>) -> &mut Self {
        self.app.world_mut().add_observer(observer);
        self
    }

    pub fn add_dependency<P: DeclarativePlugin + Default, F: Fn(&P) -> bool + 'static>(
        &mut self,
        evaluate_config: Option<F>,
    ) -> &mut Self {
        self.add_dependency_with_plugin_config(P::default(), evaluate_config);
        self
    }

    /// Add a plugin dependency to the plugin output
    pub fn add_dependency_with_plugin_config<P: DeclarativePlugin, F: Fn(&P) -> bool + 'static>(
        &mut self,
        plugin: P,
        evaluate_config: Option<F>,
    ) -> &mut Self {
        let plugin_type_id = PluginTypeId(plugin.type_id());
        let current_plugin_name = &self.working_plugin;
        let evaluate_config = move |a: &dyn DeclarativePlugin| match &evaluate_config {
            Some(f) => match <dyn Any>::downcast_ref::<P>(a) {
                Some(a) => f(a),
                _ => true,
            },
            None => true,
        };
        self.dependencies
            .push((plugin_type_id, Box::new(evaluate_config)));
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
