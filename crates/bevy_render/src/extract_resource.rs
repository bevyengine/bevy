use crate::RenderApp;

use bevy_extract::extract_base_resource::ExtractBaseResourcePlugin;
pub use bevy_render_macros::ExtractResource;

pub use bevy_extract::extract_base_resource::ExtractBaseResource;
pub use bevy_extract::extract_base_resource::ExtractResource;

// pub type ExtractResource<F : 'static + Send + Sync = ()> = ExtractBaseResource<RenderApp, F>;

pub type ExtractResourcePlugin<R, F = ()> = ExtractBaseResourcePlugin<RenderApp, R, F>;
