use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::system::{Commands, Res, Resource};

use crate::{RenderApp, RenderStage};

/// This plugin extracts the resources into the "render world".
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) step
/// for the specified [`Resource`].
pub struct ExtractResourcePlugin<R>(PhantomData<R>);

impl<R> Default for ExtractResourcePlugin<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<R: Clone + Resource> Plugin for ExtractResourcePlugin<R> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system_to_stage(RenderStage::Extract, extract_resource::<R>);
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
/// by cloning it.
pub fn extract_resource<R: Clone + Resource>(mut commands: Commands, resource: Res<R>) {
    if resource.is_changed() {
        commands.insert_resource(resource.into_inner().clone());
    }
}
