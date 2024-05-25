//! Infrastructure for recording render diagnostics.
//!
//! For more info, see [`RenderDiagnosticsPlugin`].

pub(crate) mod internal;

use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use bevy_app::{App, Plugin, PreUpdate};

use crate::RenderApp;

use self::internal::{
    sync_diagnostics, DiagnosticsRecorder, Pass, RenderDiagnosticsMutex, WriteTimestamp,
};

use super::{RenderDevice, RenderQueue};

/// Enables collecting render diagnostics, such as CPU/GPU elapsed time per render pass,
/// as well as pipeline statistics (number of primitives, number of shader invocations, etc).
///
/// To access the diagnostics, you can use [`DiagnosticsStore`](bevy_diagnostic::DiagnosticsStore) resource,
/// or add [`LogDiagnosticsPlugin`](bevy_diagnostic::LogDiagnosticsPlugin).
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
///  3. End the span, providing the same encoder.
///     ```ignore
///     time_span.end(render_context.command_encoder());
///     ```
///
/// # Supported platforms
/// Timestamp queries and pipeline statistics are currently supported only on Vulkan and DX12.
/// On other platforms (Metal, WebGPU, WebGL2) only CPU time will be recorded.
#[allow(clippy::doc_markdown)]
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

        let device = render_app.world().resource::<RenderDevice>();
        let queue = render_app.world().resource::<RenderQueue>();
        render_app.insert_resource(DiagnosticsRecorder::new(device, queue));
    }
}

/// Allows recording diagnostic spans.
pub trait RecordDiagnostics: Send + Sync {
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
        self.begin_pass_span(pass, name.into());
        PassSpanGuard {
            recorder: self,
            marker: PhantomData,
        }
    }

    #[doc(hidden)]
    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: Cow<'static, str>);

    #[doc(hidden)]
    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E);

    #[doc(hidden)]
    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: Cow<'static, str>);

    #[doc(hidden)]
    fn end_pass_span<P: Pass>(&self, pass: &mut P);
}

/// Guard returned by [`RecordDiagnostics::time_span`].
///
/// Will panic on drop unless [`TimeSpanGuard::end`] is called.
pub struct TimeSpanGuard<'a, R: ?Sized, E> {
    recorder: &'a R,
    marker: PhantomData<E>,
}

impl<R: RecordDiagnostics + ?Sized, E: WriteTimestamp> TimeSpanGuard<'_, R, E> {
    /// End the span. You have to provide the same encoder which was used to begin the span.
    pub fn end(self, encoder: &mut E) {
        self.recorder.end_time_span(encoder);
        std::mem::forget(self);
    }
}

impl<R: ?Sized, E> Drop for TimeSpanGuard<'_, R, E> {
    fn drop(&mut self) {
        panic!("TimeSpanScope::end was never called")
    }
}

/// Guard returned by [`RecordDiagnostics::pass_span`].
///
/// Will panic on drop unless [`PassSpanGuard::end`] is called.
pub struct PassSpanGuard<'a, R: ?Sized, P> {
    recorder: &'a R,
    marker: PhantomData<P>,
}

impl<R: RecordDiagnostics + ?Sized, P: Pass> PassSpanGuard<'_, R, P> {
    /// End the span. You have to provide the same encoder which was used to begin the span.
    pub fn end(self, pass: &mut P) {
        self.recorder.end_pass_span(pass);
        std::mem::forget(self);
    }
}

impl<R: ?Sized, P> Drop for PassSpanGuard<'_, R, P> {
    fn drop(&mut self) {
        panic!("PassSpanScope::end was never called")
    }
}

impl<T: RecordDiagnostics> RecordDiagnostics for Option<Arc<T>> {
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
}
