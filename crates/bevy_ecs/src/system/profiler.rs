use crate::Resources;
use downcast_rs::{impl_downcast, Downcast};
use std::borrow::Cow;

/// Runs at the start and end of each system
///
/// Profilers are used to collect diagnostics about system execution.
pub trait Profiler: Downcast + Send + Sync + 'static {
    fn start(&self, scope: Cow<'static, str>);
    fn stop(&self, scope: Cow<'static, str>);
}

pub fn profiler_start(resources: &Resources, scope: Cow<'static, str>) {
    if let Ok(profiler) = resources.get::<Box<dyn Profiler>>() {
        profiler.start(scope);
    }
}

pub fn profiler_stop(resources: &Resources, scope: Cow<'static, str>) {
    if let Ok(profiler) = resources.get::<Box<dyn Profiler>>() {
        profiler.stop(scope);
    }
}

impl_downcast!(Profiler);
