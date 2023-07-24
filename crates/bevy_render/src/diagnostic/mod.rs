pub(crate) mod internal;

use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

use bevy_app::{App, Plugin, PreUpdate};
use bevy_derive::Deref;
use bevy_diagnostic::DiagnosticId;
use bevy_ecs::system::Resource;
use bevy_utils::{AHasher, Uuid};
use smallvec::SmallVec;

use crate::RenderApp;

use self::internal::{
    sync_diagnostics, DiagnosticsRecorder, Pass, RenderDiagnosticsMutex, WriteTimestamp,
};

use super::{RenderDevice, RenderQueue};

/// Enables collecting rendering diagnostics into [`RenderDiagnostics`] resource.
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
            .init_resource::<RenderDiagnostics>()
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

/// Resource which stores rendering diagnostics of the most recent frame.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnostics(pub Vec<SpanDiagnostics>);

/// Diagnostics of a single render span.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SpanDiagnostics {
    /// Path of the span.
    pub path: SpanPath,
    /// Kind of the span.
    pub kind: SpanKind,
    /// CPU time spent during the duration of the span.
    pub elapsed_cpu: Option<Duration>,
    /// GPU time spent executing commands recorded inside the span.
    pub elapsed_gpu: Option<Duration>,
    /// Amount of times the vertex shader is ran.
    /// Accounts for the vertex cache when doing indexed rendering.
    pub vertex_shader_invocations: Option<u64>,
    /// Amount of times the clipper is invoked.
    /// This is also the amount of triangles output by the vertex shader.
    pub clipper_invocations: Option<u64>,
    /// Amount of primitives that are not culled by the clipper.
    /// This is the amount of triangles that are actually on screen and will be rasterized and rendered.
    pub clipper_primitives_out: Option<u64>,
    /// Amount of times the fragment shader is ran.
    /// Accounts for fragment shaders running in 2x2 blocks in order to get derivatives.
    pub fragment_shader_invocations: Option<u64>,
    /// Amount of times a compute shader is invoked.
    /// This will be equivalent to the dispatch count times the workgroup size.
    pub compute_shader_invocations: Option<u64>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DiagnosticKind {
    ElapsedCpu = 0,
    ElapsedGpu,
    VertexShaderInvocations,
    ClipperInvocations,
    ClipperPrimitivesOut,
    FragmentShaderInvocations,
    ComputeShaderInvocations,
}

/// Kinds of render diagnostic spans.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SpanKind {
    /// An explicit time span.
    ///
    /// See also: [`RecordDiagnostics::time_span`]
    Time,
    /// A [`wgpu::RenderPass`]. Records timestamps, as well as pipeline statistics.
    ///
    /// See also: [`RecordDiagnostics::pass_span`]
    RenderPass,
    /// A [`wgpu::ComputePass`]. Records timestamps, as well as pipeline statistics.
    ///
    /// See also: [`RecordDiagnostics::pass_span`]
    ComputePass,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
pub struct SpanName(pub Cow<'static, str>);

impl<T: Into<Cow<'static, str>>> From<T> for SpanName {
    fn from(value: T) -> Self {
        SpanName(value.into())
    }
}

#[derive(Debug, Default, Clone)]
pub struct SpanPath {
    components: SmallVec<[SpanName; 2]>,
    hash: u64,
}

impl SpanPath {
    pub fn new(components: impl IntoIterator<Item = SpanName>) -> SpanPath {
        let components: SmallVec<[SpanName; 2]> = components.into_iter().collect();
        let mut hasher = AHasher::default();
        components.hash(&mut hasher);
        let hash = hasher.finish();
        SpanPath { components, hash }
    }

    pub fn components(&self) -> impl Iterator<Item = &SpanName> + '_ {
        self.components.iter()
    }

    pub fn diagnostic_id(&self, kind: DiagnosticKind) -> DiagnosticId {
        DiagnosticId(Uuid::from_u64_pair(
            0x6140_553e_4b6a_4400 | u64::from(kind as u8),
            self.hash,
        ))
    }
}

impl Eq for SpanPath {}

impl PartialEq for SpanPath {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.components == other.components
    }
}

impl Hash for SpanPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

pub trait RecordDiagnostics: Send + Sync {
    fn time_span<E: WriteTimestamp, N: Into<SpanName>>(
        &self,
        encoder: &mut E,
        name: N,
    ) -> TimeSpanScope<'_, Self, E> {
        self.begin_time_span(encoder, name.into());
        TimeSpanScope {
            recorder: self,
            marker: PhantomData,
        }
    }

    fn pass_span<P: Pass, N: Into<SpanName>>(
        &self,
        pass: &mut P,
        name: N,
    ) -> PassSpanScope<'_, Self, P> {
        self.begin_pass_span(pass, name.into());
        PassSpanScope {
            recorder: self,
            marker: PhantomData,
        }
    }

    #[doc(hidden)]
    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: SpanName);

    #[doc(hidden)]
    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E);

    #[doc(hidden)]
    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: SpanName);

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
    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: SpanName) {
        if let Some(recorder) = &self {
            recorder.begin_time_span(encoder, name);
        }
    }

    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        if let Some(recorder) = &self {
            recorder.end_time_span(encoder);
        }
    }

    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: SpanName) {
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
