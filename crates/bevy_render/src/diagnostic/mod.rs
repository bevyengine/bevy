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

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(render_diagnostics_mutex);
        }
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let device = render_app.world.resource::<RenderDevice>();
        let queue = render_app.world.resource::<RenderQueue>();
        render_app.insert_resource(DiagnosticsRecorder::new(device, queue));
    }
}

pub trait RecordDiagnostics: Send + Sync {
    fn time_span<E, N>(&self, encoder: &mut E, name: N) -> TimeSpanScope<'_, Self, E>
    where
        E: WriteTimestamp,
        N: Into<Cow<'static, str>>,
    {
        self.begin_time_span(encoder, name.into());
        TimeSpanScope {
            recorder: self,
            marker: PhantomData,
        }
    }

    fn pass_span<P, N>(&self, pass: &mut P, name: N) -> PassSpanScope<'_, Self, P>
    where
        P: Pass,
        N: Into<Cow<'static, str>>,
    {
        self.begin_pass_span(pass, name.into());
        PassSpanScope {
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

pub struct TimeSpanScope<'a, R: ?Sized, E> {
    recorder: &'a R,
    marker: PhantomData<E>,
}

impl<R: RecordDiagnostics + ?Sized, E: WriteTimestamp> TimeSpanScope<'_, R, E> {
    pub fn end(self, encoder: &mut E) {
        self.recorder.end_time_span(encoder);
        std::mem::forget(self);
    }
}

impl<R: ?Sized, E> Drop for TimeSpanScope<'_, R, E> {
    fn drop(&mut self) {
        panic!("TimeSpanScope::end was never called")
    }
}

pub struct PassSpanScope<'a, R: ?Sized, P> {
    recorder: &'a R,
    marker: PhantomData<P>,
}

impl<R: RecordDiagnostics + ?Sized, P: Pass> PassSpanScope<'_, R, P> {
    pub fn end(self, pass: &mut P) {
        self.recorder.end_pass_span(pass);
        std::mem::forget(self);
    }
}

impl<R: ?Sized, P> Drop for PassSpanScope<'_, R, P> {
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
