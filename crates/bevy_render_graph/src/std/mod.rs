use bevy_app::{PluginGroup, PluginGroupBuilder};

mod builder;
mod copy;
mod fullscreen;
mod misc;
mod swap;

pub use builder::*;
pub use copy::*;
pub use fullscreen::*;
pub use misc::*;
pub use swap::*;

///A set of minimal plugins that setup assets and behavior for the graph standard library.
pub struct DefaultRenderGraphPlugins;

impl PluginGroup for DefaultRenderGraphPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(FullscreenPlugin)
    }
}
