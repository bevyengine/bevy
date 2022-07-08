use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::system::{Commands, Res, Resource};
pub use bevy_render_macros::ExtractResource;

use crate::{Extract, RenderApp, RenderStage};

/// Describes how a resource gets extracted for rendering.
///
/// Therefore the resource is transferred from the "main world" into the "render world"
/// in the [`RenderStage::Extract`](crate::RenderStage::Extract) step.
pub trait ExtractResource: Resource {
    type Source: Resource;

    /// Defines how the resource is transferred into the "render world".
    fn extract_resource(source: &Self::Source) -> Self;
}

/// This plugin extracts the resources into the "render world".
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) step
/// for the specified [`Resource`].
pub struct ExtractResourcePlugin<R: ExtractResource>(PhantomData<R>);

impl<R: ExtractResource> Default for ExtractResourcePlugin<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<R: ExtractResource> Plugin for ExtractResourcePlugin<R> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system_to_stage(RenderStage::Extract, extract_resource::<R>);
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
/// by cloning it.
pub fn extract_resource<R: ExtractResource>(
    mut commands: Commands,
    resource: Extract<Res<R::Source>>,
) {
    if resource.is_changed() {
        commands.insert_resource(R::extract_resource(&*resource));
    }
}
