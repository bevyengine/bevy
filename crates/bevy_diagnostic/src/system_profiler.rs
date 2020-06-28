use crate::{Diagnostic, DiagnosticId, Diagnostics};
use legion::{
    systems::{profiler::Profiler, Res, ResMut},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant, borrow::Cow,
};

#[derive(Debug)]
struct SystemRunInfo {
    start: Instant,
    stop: Instant,
}

#[derive(Default)]
struct SystemProfiles {
    diagnostic_id: DiagnosticId,
    history: Vec<SystemRunInfo>,
    current_start: Option<Instant>,
}

#[derive(Default)]
pub struct SystemProfiler {
    system_profiles: Arc<RwLock<HashMap<Cow<'static, str>, SystemProfiles>>>,
}

impl Profiler for SystemProfiler {
    fn start(&self, scope: Cow<'static, str>) {
        let mut system_profiles = self.system_profiles.write().unwrap();
        let profiles = system_profiles
            .entry(scope.clone())
            .or_insert_with(|| SystemProfiles::default());

        profiles.current_start = Some(Instant::now());
    }

    fn stop(&self, scope: Cow<'static, str>) {
        let now = Instant::now();
        let mut system_profiles = self.system_profiles.write().unwrap();
        let profiles = system_profiles.get_mut(&scope).unwrap();
        if let Some(current_start) = profiles.current_start.take() {
            profiles.history.push(SystemRunInfo {
                start: current_start,
                stop: now,
            });
        }
    }
}

pub fn profiler_diagnostic_system(
    mut diagnostics: ResMut<Diagnostics>,
    system_profiler: Res<Box<dyn Profiler>>,
) {
    let system_profiler = system_profiler.downcast_ref::<SystemProfiler>().unwrap();
    let mut system_profiles = system_profiler.system_profiles.write().unwrap();
    for (scope, profiles) in system_profiles.iter_mut() {
        if diagnostics.get(profiles.diagnostic_id).is_none() {
            diagnostics.add(Diagnostic::new(
                profiles.diagnostic_id,
                &scope,
                20,
            ))
        }
        for profile in profiles.history.drain(..) {
            diagnostics.add_measurement(
                profiles.diagnostic_id,
                (profile.stop - profile.start).as_secs_f64(),
            );
        }
    }
}
