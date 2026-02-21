
use crate::RenderApp;

use bevy_ecs::resource::Resource;
pub use bevy_render_macros::ExtractResource;
use bevy_extract::extract_base_resource::ExtractBaseResourcePlugin;

pub use bevy_extract::extract_base_resource::ExtractBaseResource;

// pub type ExtractResource<F : 'static + Send + Sync = ()> = ExtractBaseResource<RenderApp, F>;

pub type ExtractResourcePlugin<R, F  = ()> = ExtractBaseResourcePlugin<RenderApp, R, F>;
