use crate::{
    render_resource::{GpuArrayBuffer, GpuArrayBufferable},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_app::{App, AppLabel, InternedAppLabel, Plugin};
use bevy_ecs::{
    prelude::{Component, Entity},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
};
use std::marker::PhantomData;

/// This plugin prepares the components of the corresponding type for the GPU
/// by storing them in a [`GpuArrayBuffer`].
pub struct GpuComponentArrayBufferPlugin<C: Component + GpuArrayBufferable>(PhantomData<C>);

impl<C: Component + GpuArrayBufferable> Plugin for GpuComponentArrayBufferPlugin<C> {
    fn require_sub_apps(&self) -> Vec<InternedAppLabel> {
        vec![RenderApp.intern()]
    }

    fn ready(&self, app: &App) -> bool {
        app.contains_resource::<RenderDevice>()
    }

    fn finalize(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(GpuArrayBuffer::<C>::new(
                render_app.world().resource::<RenderDevice>(),
            ))
            .add_systems(
                Render,
                prepare_gpu_component_array_buffers::<C>.in_set(RenderSet::PrepareResources),
            );
    }
}

impl<C: Component + GpuArrayBufferable> Default for GpuComponentArrayBufferPlugin<C> {
    fn default() -> Self {
        Self(PhantomData::<C>)
    }
}

fn prepare_gpu_component_array_buffers<C: Component + GpuArrayBufferable>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut gpu_array_buffer: ResMut<GpuArrayBuffer<C>>,
    components: Query<(Entity, &C)>,
) {
    gpu_array_buffer.clear();

    let entities = components
        .iter()
        .map(|(entity, component)| (entity, gpu_array_buffer.push(component.clone())))
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    gpu_array_buffer.write_buffer(&render_device, &render_queue);
}
