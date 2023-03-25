use crate::{
    render_resource::{GpuList, GpuListable},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{
    prelude::{Component, Entity},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
};
use std::marker::PhantomData;

/// This plugin prepares the components of the corresponding type for the GPU
/// by storing them in a [`GpuList`].
pub struct GpuComponentListPlugin<C: Component + GpuListable>(PhantomData<C>);

impl<C: Component + GpuListable> Plugin for GpuComponentListPlugin<C> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(GpuList::<C>::new(
                    render_app.world.resource::<RenderDevice>(),
                ))
                .add_systems(
                    Render,
                    prepare_gpu_component_lists::<C>.in_set(RenderSet::Prepare),
                );
        }
    }
}

impl<C: Component + GpuListable> Default for GpuComponentListPlugin<C> {
    fn default() -> Self {
        Self(PhantomData::<C>)
    }
}

fn prepare_gpu_component_lists<C: Component + GpuListable>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut gpu_list: ResMut<GpuList<C>>,
    components: Query<(Entity, &C)>,
) {
    gpu_list.clear();

    let entities = components
        .iter()
        .map(|(entity, component)| (entity, gpu_list.push(component.clone())))
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    gpu_list.write_buffer(&render_device, &render_queue);
}
