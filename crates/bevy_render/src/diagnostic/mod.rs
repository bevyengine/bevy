//! Infrastructure for recording render diagnostics.
//!
//! For more info, see [`RenderDiagnosticsPlugin`].

mod erased_render_asset_diagnostic_plugin;
pub(crate) mod internal;
mod mesh_allocator_diagnostic_plugin;
mod render_asset_diagnostic_plugin;
#[cfg(feature = "tracing-tracy")]
mod tracy_gpu;

use alloc::{borrow::Cow, sync::Arc};
use bevy_ecs::{
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use core::marker::PhantomData;
use wgpu::{
    BufferSlice, CommandEncoder, ComputePassTimestampWrites, QuerySet, RenderPassTimestampWrites,
};

use bevy_app::{App, Plugin, PreUpdate};

use crate::{
    renderer::{PendingCommandBuffers, RenderGraph, RenderGraphSystems},
    GpuResourceAppExt, Render, RenderApp, RenderSystems,
};

use self::internal::{sync_diagnostics, Pass, RenderDiagnosticsMutex, WriteTimestamp};
pub use self::{
    erased_render_asset_diagnostic_plugin::ErasedRenderAssetDiagnosticPlugin,
    internal::{DiagnosticsRecorder, PassKind},
    mesh_allocator_diagnostic_plugin::MeshAllocatorDiagnosticPlugin,
    render_asset_diagnostic_plugin::RenderAssetDiagnosticPlugin,
};

use crate::renderer::RenderDevice;

/// Enables collecting render diagnostics, such as CPU/GPU elapsed time per render pass,
/// as well as pipeline statistics (number of primitives, number of shader invocations, etc).
///
/// To access the diagnostics, you can use the [`DiagnosticsStore`](bevy_diagnostic::DiagnosticsStore) resource,
/// add [`LogDiagnosticsPlugin`](bevy_diagnostic::LogDiagnosticsPlugin), or use [Tracy](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md#tracy-renderqueue).
///
/// To record diagnostics in your own passes:
///  1. First, obtain the diagnostic recorder using [`RenderContext::diagnostic_recorder`](crate::renderer::RenderContext::diagnostic_recorder).
///
///     It won't do anything unless [`RenderDiagnosticsPlugin`] is present,
///     so you're free to omit `#[cfg]` clauses.
///     ```ignore
///     let diagnostics = render_context.diagnostic_recorder();
///     ```
///  2. Begin the span inside a command encoder, or a render/compute pass encoder.
///     ```ignore
///     let time_span = diagnostics.time_span(render_context.command_encoder(), "shadows");
///     ```
///  3. End the span, providing the encoder (or the same render/compute pass).
///     ```ignore
///     time_span.end(render_context.command_encoder());
///     ```
///
/// # Supported platforms
/// Timestamp queries and pipeline statistics are supported when the backend
/// exposes the corresponding wgpu features. Metal supports whole-pass GPU
/// timestamps through pass descriptor boundary writes.
#[derive(Default)]
pub struct RenderDiagnosticsPlugin;

impl Plugin for RenderDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        let render_diagnostics_mutex = RenderDiagnosticsMutex::default();
        app.insert_resource(render_diagnostics_mutex.clone())
            .add_systems(PreUpdate, sync_diagnostics);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(render_diagnostics_mutex);
        }
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_gpu_resource::<DiagnosticsRecorder>();

        render_app
            .add_systems(
                Render,
                begin_diagnostics_frame
                    .after(RenderSystems::ExtractCommands)
                    .before(RenderSystems::PrepareAssets),
            )
            .add_systems(
                RenderGraph,
                (
                    resolve_encoder
                        .after(RenderGraphSystems::Render)
                        .before(RenderGraphSystems::Submit),
                    finish_diagnostics_frame.in_set(RenderGraphSystems::Finish),
                ),
            );
    }
}

impl FromWorld for DiagnosticsRecorder {
    fn from_world(world: &mut World) -> Self {
        DiagnosticsRecorder::new(world.resource(), world.resource(), world.resource())
    }
}

/// Starts the diagnostics recorder for the frame.
pub fn begin_diagnostics_frame(mut recorder: ResMut<DiagnosticsRecorder>) {
    recorder.begin_frame();
}

/// Resolves the encoder used for diagnostic recording
pub fn resolve_encoder(
    mut recorder: ResMut<DiagnosticsRecorder>,
    render_device: Res<RenderDevice>,
    mut pending_buffers: ResMut<PendingCommandBuffers>,
) {
    let mut encoder =
        render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    recorder.resolve(&mut encoder);
    pending_buffers.push_encoder(encoder, "resolve_diagnostics");
}

/// Ends the current frame for the diagnostics recorder and syncs it with the main world.
fn finish_diagnostics_frame(
    mut recorder: ResMut<DiagnosticsRecorder>,
    render_device: Res<RenderDevice>,
    mutex: Res<RenderDiagnosticsMutex>,
) {
    let mutex = mutex.0.clone();
    recorder.finish_frame(&render_device, move |diagnostics| {
        *mutex.lock().unwrap() = Some(diagnostics);
    });
}

/// Allows recording diagnostic spans.
pub trait RecordDiagnostics: Send + Sync {
    /// Prepares a pass diagnostic span before its descriptor is constructed.
    ///
    /// This follows the same begin/end model as [`RecordDiagnostics::pass_span`],
    /// while also supplying automatic descriptor timestamps on backends that
    /// cannot write timestamps inside an active pass.
    fn pass_span_descriptor<N>(&self, kind: PassKind, name: N) -> PassDescriptorSpan<'_, Self>
    where
        N: Into<Cow<'static, str>>,
    {
        let name = name.into();
        let timestamps = self.begin_pass_boundary_fallback(kind, name.clone());
        PassDescriptorSpan {
            recorder: self,
            name,
            timestamps,
        }
    }

    /// Begin a time span, which will record elapsed CPU and GPU time.
    ///
    /// Returns a guard, which will panic on drop unless you end the span.
    fn time_span<E, N>(&self, encoder: &mut E, name: N) -> TimeSpanGuard<'_, Self, E>
    where
        E: WriteTimestamp,
        N: Into<Cow<'static, str>>,
    {
        self.begin_time_span(encoder, name.into());
        TimeSpanGuard {
            recorder: self,
            marker: PhantomData,
        }
    }

    /// Begin a pass span, which will record elapsed CPU and GPU time,
    /// as well as pipeline statistics on supported platforms.
    ///
    /// Returns a guard, which will panic on drop unless you end the span.
    fn pass_span<P, N>(&self, pass: &mut P, name: N) -> PassSpanGuard<'_, Self, P>
    where
        P: Pass,
        N: Into<Cow<'static, str>>,
    {
        let name = name.into();
        self.begin_pass_span(pass, name.clone());
        PassSpanGuard {
            recorder: self,
            name,
            marker: PhantomData,
        }
    }

    /// Reads an `f32` from the specified buffer and uploads it as a diagnostic.
    ///
    /// The provided buffer slice must be 4 bytes long, and the buffer must have [`wgpu::BufferUsages::COPY_SRC`];
    fn record_f32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>;

    /// Reads a `u32` from the specified buffer and uploads it as a diagnostic.
    ///
    /// The provided buffer slice must be 4 bytes long, and the buffer must have [`wgpu::BufferUsages::COPY_SRC`];
    fn record_u32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>;

    #[doc(hidden)]
    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: Cow<'static, str>);

    #[doc(hidden)]
    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E);

    #[doc(hidden)]
    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: Cow<'static, str>);

    #[doc(hidden)]
    fn end_pass_span<P: Pass>(&self, pass: &mut P);

    #[doc(hidden)]
    fn begin_pass_boundary_fallback(
        &self,
        kind: PassKind,
        name: Cow<'static, str>,
    ) -> Option<PassBoundaryTimestamps>;

    #[doc(hidden)]
    fn end_pass_boundary_fallback(&self);
}

/// Query indices reserved for automatic beginning/end timestamp writes on a
/// render or compute pass descriptor.
#[doc(hidden)]
pub struct PassBoundaryTimestamps {
    query_set: QuerySet,
    beginning_of_pass_write_index: u32,
    end_of_pass_write_index: u32,
}

/// A pass diagnostic span prepared before constructing its pass descriptor.
pub struct PassDescriptorSpan<'a, R: ?Sized> {
    recorder: &'a R,
    name: Cow<'static, str>,
    timestamps: Option<PassBoundaryTimestamps>,
}

impl<'a, R: RecordDiagnostics + ?Sized> PassDescriptorSpan<'a, R> {
    /// Returns timestamp writes suitable for a [`wgpu::RenderPassDescriptor`].
    pub fn render_timestamp_writes(&self) -> Option<RenderPassTimestampWrites<'_>> {
        self.timestamps
            .as_ref()
            .map(|timestamps| RenderPassTimestampWrites {
                query_set: &timestamps.query_set,
                beginning_of_pass_write_index: Some(timestamps.beginning_of_pass_write_index),
                end_of_pass_write_index: Some(timestamps.end_of_pass_write_index),
            })
    }

    /// Returns timestamp writes suitable for a [`wgpu::ComputePassDescriptor`].
    pub fn compute_timestamp_writes(&self) -> Option<ComputePassTimestampWrites<'_>> {
        self.timestamps
            .as_ref()
            .map(|timestamps| ComputePassTimestampWrites {
                query_set: &timestamps.query_set,
                beginning_of_pass_write_index: Some(timestamps.beginning_of_pass_write_index),
                end_of_pass_write_index: Some(timestamps.end_of_pass_write_index),
            })
    }

    /// Begins recording after the pass has been constructed.
    pub fn begin<P: Pass>(self, pass: &mut P) -> DescriptorPassSpanGuard<'a, R, P> {
        let uses_descriptor_timestamps = self.timestamps.is_some();
        if !uses_descriptor_timestamps {
            self.recorder.begin_pass_span(pass, self.name.clone());
        }

        let guard = DescriptorPassSpanGuard {
            recorder: self.recorder,
            name: self.name.clone(),
            uses_descriptor_timestamps,
            marker: PhantomData,
        };
        core::mem::forget(self);
        guard
    }
}

impl<R: ?Sized> Drop for PassDescriptorSpan<'_, R> {
    fn drop(&mut self) {
        if self.timestamps.is_some() {
            panic!(
                "PassDescriptorSpan::begin was never called for {}",
                self.name
            );
        }
    }
}

/// Guard returned by [`PassDescriptorSpan::begin`].
pub struct DescriptorPassSpanGuard<'a, R: ?Sized, P> {
    recorder: &'a R,
    name: Cow<'static, str>,
    uses_descriptor_timestamps: bool,
    marker: PhantomData<P>,
}

impl<R: RecordDiagnostics + ?Sized, P: Pass> DescriptorPassSpanGuard<'_, R, P> {
    /// Ends the pass diagnostic span.
    pub fn end(self, pass: &mut P) {
        if self.uses_descriptor_timestamps {
            self.recorder.end_pass_boundary_fallback();
        } else {
            self.recorder.end_pass_span(pass);
        }
        core::mem::forget(self);
    }
}

impl<R: ?Sized, P> Drop for DescriptorPassSpanGuard<'_, R, P> {
    fn drop(&mut self) {
        panic!(
            "DescriptorPassSpanGuard::end was never called for {}",
            self.name
        );
    }
}

/// Guard returned by [`RecordDiagnostics::time_span`].
///
/// Will panic on drop unless [`TimeSpanGuard::end`] is called.
pub struct TimeSpanGuard<'a, R: ?Sized, E> {
    recorder: &'a R,
    marker: PhantomData<E>,
}

impl<R: RecordDiagnostics + ?Sized, E: WriteTimestamp> TimeSpanGuard<'_, R, E> {
    /// End the span.
    pub fn end(self, encoder: &mut E) {
        self.recorder.end_time_span(encoder);
        core::mem::forget(self);
    }
}

impl<R: ?Sized, E> Drop for TimeSpanGuard<'_, R, E> {
    fn drop(&mut self) {
        bevy_log::error!("TimeSpanScope::end was never called");
    }
}

/// Guard returned by [`RecordDiagnostics::pass_span`].
///
/// Will panic on drop unless [`PassSpanGuard::end`] is called.
pub struct PassSpanGuard<'a, R: ?Sized, P> {
    recorder: &'a R,
    name: Cow<'static, str>,
    marker: PhantomData<P>,
}

impl<R: RecordDiagnostics + ?Sized, P: Pass> PassSpanGuard<'_, R, P> {
    /// End the span. You have to provide the same pass which was used to begin the span.
    pub fn end(self, pass: &mut P) {
        self.recorder.end_pass_span(pass);
        core::mem::forget(self);
    }
}

impl<R: ?Sized, P> Drop for PassSpanGuard<'_, R, P> {
    fn drop(&mut self) {
        panic!("PassSpanGuard::end was never called for {}", self.name)
    }
}

impl<T: RecordDiagnostics> RecordDiagnostics for Option<Arc<T>> {
    fn record_f32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>,
    {
        if let Some(recorder) = &self {
            recorder.record_f32(command_encoder, buffer, name);
        }
    }

    fn record_u32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>,
    {
        if let Some(recorder) = &self {
            recorder.record_u32(command_encoder, buffer, name);
        }
    }

    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: Cow<'static, str>) {
        if let Some(recorder) = &self {
            recorder.begin_time_span(encoder, name);
        }
    }

    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        if let Some(recorder) = &self {
            recorder.end_time_span(encoder);
        }
    }

    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: Cow<'static, str>) {
        if let Some(recorder) = &self {
            recorder.begin_pass_span(pass, name);
        }
    }

    fn end_pass_span<P: Pass>(&self, pass: &mut P) {
        if let Some(recorder) = &self {
            recorder.end_pass_span(pass);
        }
    }

    fn begin_pass_boundary_fallback(
        &self,
        kind: PassKind,
        name: Cow<'static, str>,
    ) -> Option<PassBoundaryTimestamps> {
        self.as_ref()
            .and_then(|recorder| recorder.begin_pass_boundary_fallback(kind, name))
    }

    fn end_pass_boundary_fallback(&self) {
        if let Some(recorder) = self {
            recorder.end_pass_boundary_fallback();
        }
    }
}

impl<'a, T: RecordDiagnostics> RecordDiagnostics for Option<&'a T> {
    fn record_f32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>,
    {
        if let Some(recorder) = self {
            recorder.record_f32(command_encoder, buffer, name);
        }
    }

    fn record_u32<N>(&self, command_encoder: &mut CommandEncoder, buffer: &BufferSlice, name: N)
    where
        N: Into<Cow<'static, str>>,
    {
        if let Some(recorder) = self {
            recorder.record_u32(command_encoder, buffer, name);
        }
    }

    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: Cow<'static, str>) {
        if let Some(recorder) = self {
            recorder.begin_time_span(encoder, name);
        }
    }

    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        if let Some(recorder) = self {
            recorder.end_time_span(encoder);
        }
    }

    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: Cow<'static, str>) {
        if let Some(recorder) = self {
            recorder.begin_pass_span(pass, name);
        }
    }

    fn end_pass_span<P: Pass>(&self, pass: &mut P) {
        if let Some(recorder) = self {
            recorder.end_pass_span(pass);
        }
    }

    fn begin_pass_boundary_fallback(
        &self,
        kind: PassKind,
        name: Cow<'static, str>,
    ) -> Option<PassBoundaryTimestamps> {
        self.and_then(|recorder| recorder.begin_pass_boundary_fallback(kind, name))
    }

    fn end_pass_boundary_fallback(&self) {
        if let Some(recorder) = self {
            recorder.end_pass_boundary_fallback();
        }
    }
}
