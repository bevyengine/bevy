use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::system::{
    lifetimeless::{SCommands, SRes},
    Resource, RunSystem, SystemParamItem,
};

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
        let system = ExtractResourceSystem::<R>::system(&mut app.world);
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system_to_stage(RenderStage::Extract, system);
        }
    }
}

/// This system extracts the resource of the corresponding [`Resource`] type
/// by cloning it.
pub struct ExtractResourceSystem<R: Clone + Resource>(PhantomData<R>);

impl<R: Clone + Resource> RunSystem for ExtractResourceSystem<R> {
    type Param = (SCommands, SRes<R>);

    fn run((mut commands, res): SystemParamItem<Self::Param>) {
        if res.is_added() || res.is_changed() {
            commands.insert_resource(res.into_inner().clone());
        }
    }
}
