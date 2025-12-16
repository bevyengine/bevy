use crate::diagnostic::internal::DiagnosticsRecorder;
use crate::render_phase::TrackedRenderPass;
use crate::render_resource::{CommandEncoder, RenderPassDescriptor};
use crate::renderer::RenderDevice;
use bevy_ecs::change_detection::Tick;
use bevy_ecs::component::ComponentId;
use bevy_ecs::prelude::*;
use bevy_ecs::query::{FilteredAccessSet, QueryData, QueryFilter, QueryState};
use bevy_ecs::system::{
    Deferred, SystemBuffer, SystemMeta, SystemParam, SystemParamValidationError,
};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::world::DeferredWorld;
use bevy_tasks::ComputeTaskPool;
use core::marker::PhantomData;
use tracing::info_span;
use wgpu::CommandBuffer;

#[derive(Resource, Default)]
pub struct PendingCommandBuffers {
    buffers: Vec<CommandBuffer>,
    encoders: Vec<CommandEncoder>,
}

impl PendingCommandBuffers {
    pub fn push(&mut self, buffers: impl IntoIterator<Item = CommandBuffer>) {
        self.buffers.extend(buffers);
    }

    pub fn push_encoder(&mut self, encoder: CommandEncoder) {
        self.encoders.push(encoder);
    }

    pub fn take(&mut self) -> Vec<CommandBuffer> {
        if !self.encoders.is_empty() {
            let _span = info_span!("finish_encoders", count = self.encoders.len()).entered();
            let encoders = core::mem::take(&mut self.encoders);
            let task_pool = ComputeTaskPool::get();
            let finished: Vec<CommandBuffer> = task_pool.scope(|scope| {
                for encoder in encoders {
                    scope.spawn(async move { encoder.finish() });
                }
            });
            self.buffers.extend(finished);
        }
        core::mem::take(&mut self.buffers)
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty() && self.encoders.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffers.len() + self.encoders.len()
    }
}

#[derive(Default)]
pub struct RenderContextState {
    command_encoder: Option<CommandEncoder>,
    command_buffers: Vec<CommandBuffer>,
    render_device: Option<RenderDevice>,
}

impl RenderContextState {
    fn flush_encoder(&mut self) {
        if let Some(encoder) = self.command_encoder.take() {
            self.command_buffers.push(encoder.finish());
        }
    }

    fn command_encoder(&mut self) -> &mut CommandEncoder {
        let render_device = self
            .render_device
            .as_ref()
            .expect("RenderDevice must be set before accessing command_encoder");

        self.command_encoder.get_or_insert_with(|| {
            render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        })
    }

    pub fn finish(&mut self) -> Vec<CommandBuffer> {
        self.flush_encoder();
        core::mem::take(&mut self.command_buffers)
    }
}

impl SystemBuffer for RenderContextState {
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        let _span = info_span!("RenderContextState::apply", system = %system_meta.name()).entered();

        let has_buffers = !self.command_buffers.is_empty();
        let has_encoder = self.command_encoder.is_some();

        if has_buffers || has_encoder {
            let mut pending = world.resource_mut::<PendingCommandBuffers>();

            if has_buffers {
                pending.push(core::mem::take(&mut self.command_buffers));
            }

            if let Some(encoder) = self.command_encoder.take() {
                pending.push_encoder(encoder);
            }
        }

        self.render_device = None;
    }

    fn queue(&mut self, _system_meta: &SystemMeta, _world: DeferredWorld) {}
}

#[derive(SystemParam)]
pub struct RenderContext<'w, 's> {
    state: Deferred<'s, RenderContextState>,
    render_device: Res<'w, RenderDevice>,
    diagnostics_recorder: Option<Res<'w, DiagnosticsRecorder>>,
}

impl<'w, 's> RenderContext<'w, 's> {
    fn ensure_device(&mut self) {
        if self.state.render_device.is_none() {
            self.state.render_device = Some(self.render_device.clone());
        }
    }

    pub fn render_device(&self) -> &RenderDevice {
        &self.render_device
    }

    pub fn diagnostic_recorder(&self) -> Option<Res<'w, DiagnosticsRecorder>> {
        self.diagnostics_recorder.as_ref().map(Res::clone)
    }

    pub fn command_encoder(&mut self) -> &mut CommandEncoder {
        self.ensure_device();
        self.state.command_encoder()
    }

    pub fn begin_tracked_render_pass<'a>(
        &'a mut self,
        descriptor: RenderPassDescriptor<'_>,
    ) -> TrackedRenderPass<'a> {
        self.ensure_device();

        let command_encoder = self.state.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        });

        let render_pass = command_encoder.begin_render_pass(&descriptor);
        TrackedRenderPass::new(&self.render_device, render_pass)
    }

    pub fn add_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.state.flush_encoder();
        self.state.command_buffers.push(command_buffer);
    }
}

#[derive(SystemParam)]
pub struct FlushCommands<'w> {
    pending: ResMut<'w, PendingCommandBuffers>,
    queue: Res<'w, super::RenderQueue>,
}

impl<'w> FlushCommands<'w> {
    pub fn flush(&mut self) {
        let buffers = self.pending.take();
        if !buffers.is_empty() {
            self.queue.submit(buffers);
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentViewEntity(pub Entity);

impl CurrentViewEntity {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.0
    }
}

#[derive(SystemParam)]
pub struct CurrentView<'w> {
    entity: Res<'w, CurrentViewEntity>,
}

impl<'w> CurrentView<'w> {
    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity.0
    }
}

impl<'w> core::ops::Deref for CurrentView<'w> {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entity.0
    }
}

pub struct ViewQuery<'w, 's, D: QueryData, F: QueryFilter = ()> {
    entity: Entity,
    item: D::Item<'w, 's>,
    _filter: PhantomData<F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> ViewQuery<'w, 's, D, F> {
    #[inline]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn into_inner(self) -> D::Item<'w, 's> {
        self.item
    }
}

pub struct ViewQueryState<D: QueryData, F: QueryFilter> {
    resource_id: ComponentId,
    query_state: QueryState<D, F>,
}

// SAFETY: ViewQuery accesses the CurrentViewEntity resource (read) and query components.
// Access is properly registered in init_access.
unsafe impl<'a, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for ViewQuery<'a, '_, D, F>
{
    type State = ViewQueryState<D, F>;
    type Item<'w, 's> = ViewQuery<'w, 's, D, F>;

    fn init_state(world: &mut World) -> Self::State {
        ViewQueryState {
            resource_id: world
                .components_registrator()
                .register_resource::<CurrentViewEntity>(),
            query_state: QueryState::new(world),
        }
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        component_access_set.add_unfiltered_resource_read(state.resource_id);

        <Query<'_, '_, D, F> as SystemParam>::init_access(
            &state.query_state,
            system_meta,
            component_access_set,
            world,
        );
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // Check if CurrentViewEntity resource exists and get the entity
        // SAFETY: We have registered resource read access in init_access
        let current_view = unsafe { world.get_resource::<CurrentViewEntity>() };

        let Some(current_view) = current_view else {
            return Err(SystemParamValidationError::skipped::<Self>(
                "CurrentViewEntity resource not present",
            ));
        };

        let entity = current_view.entity();

        // Check if the current view entity matches the query
        // SAFETY: Query state access is properly registered in init_access.
        // The caller ensures the world matches the one used in init_state.
        let result = unsafe { state.query_state.get_unchecked(world, entity) };

        if result.is_err() {
            return Err(SystemParamValidationError::skipped::<Self>(
                "Current view entity does not match query",
            ));
        }

        Ok(())
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: We have registered resource read access and validate_param succeeded
        let current_view = unsafe {
            world
                .get_resource::<CurrentViewEntity>()
                .expect("CurrentViewEntity must exist")
        };

        let entity = current_view.entity();

        // SAFETY: Query state access is properly registered in init_access.
        // validate_param verified the entity matches.
        let item = unsafe {
            state
                .query_state
                .get_unchecked(world, entity)
                .expect("view entity must match query")
        };

        ViewQuery {
            entity,
            item,
            _filter: PhantomData,
        }
    }
}

// SAFETY: ViewQuery with ReadOnlyQueryData only reads from the world.
unsafe impl<'w, 's, D: bevy_ecs::query::ReadOnlyQueryData + 'static, F: QueryFilter + 'static>
    bevy_ecs::system::ReadOnlySystemParam for ViewQuery<'w, 's, D, F>
{
}
