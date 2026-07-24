use core::
    any::Any
;


pub(crate) mod approval;
pub(crate) mod erased_resource;
pub(crate) mod graph;
pub(crate) mod metadata_ptr;
/// Declarative Plugin public API.
pub mod plugin_data;
pub use plugin_data::PluginOutput;

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
