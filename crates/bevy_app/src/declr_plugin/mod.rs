use core::any::Any;

pub(crate) mod approval;
pub(crate) mod erased;
pub(crate) mod graph;
pub(crate) mod metadata_ptr;
/// Declarative Plugin public API.
pub mod plugin_data;
pub use plugin_data::PluginOutput;

use crate::{
    graph::{PluginList, PluginRegistrationGraph},
    App,
};

/// A declarative alternative to [`Plugin`]
pub trait DeclarativePlugin: Any {
    /// Plugin registration function.
    fn build(&self, output: &mut PluginOutput);

    /// When this is a zero-sized type, it will give the same [`PluginOutput`]
    /// every time [`DeclarativePlugin`] is called.
    fn zero_sized_instances_are_identical(&self) -> bool {
        true
    }
}

pub(crate) trait DeclrAppExt {
    fn apply_declarative(&mut self, entry_point_list: PluginList) -> Result<(), ()>;
}

impl DeclrAppExt for App {
    fn apply_declarative(&mut self, entry_point_list: PluginList) -> Result<(), ()> {
        let graph = entry_point_list.expand()?;
        if graph.can_build() {
            match graph.try_build() {
                Ok(graph_2) => {}
                Err(_) => todo!(),
            }
        }
        Ok(())
    }
}
