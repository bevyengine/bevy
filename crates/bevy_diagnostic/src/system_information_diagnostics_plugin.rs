use crate::{Diagnostic, DiagnosticId, Diagnostics, BYTES_TO_GIB};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{ResMut, Resource};
use sysinfo::{CpuExt, System, SystemExt};

#[derive(Resource)]
pub struct SystemInfoResource {
    pub sys: System,
}

impl Default for SystemInfoResource {
    fn default() -> Self {
        SystemInfoResource {
            sys: System::new_all(),
        }
    }
}

/// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
///
/// Supported targets:
/// * linux,
/// * windows,
/// * android,
/// * macos
///
/// NOT supported when using the `bevy/dynamic` feature even when using previously mentioned targets
#[derive(Default)]
pub struct SystemInformationDiagnosticsPlugin;

impl Plugin for SystemInformationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: sysinfo fails to compile when using bevy dynamic or on iOS and does nothing on wasm
        #[cfg(all(
            any(
                target_os = "linux",
                target_os = "windows",
                target_os = "android",
                target_os = "macos"
            ),
            not(feature = "bevy_dynamic_plugin")
        ))]
        {
            app.insert_resource(SystemInfoResource::default());
            app.add_startup_system(Self::setup_system)
                .add_system(Self::diagnostic_system);
        }
        #[cfg(not(all(
            any(
                target_os = "linux",
                target_os = "windows",
                target_os = "android",
                target_os = "macos"
            ),
            not(feature = "bevy_dynamic_plugin")
        )))]
        {
            bevy_log::warn!("This platform and/or configuration is not supported!")
        }
    }
}

impl SystemInformationDiagnosticsPlugin {
    pub const CPU_USAGE: DiagnosticId =
        DiagnosticId::from_u128(78494871623549551581510633532637320956);
    pub const MEM_USAGE: DiagnosticId =
        DiagnosticId::from_u128(42846254859293759601295317811892519825);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::CPU_USAGE, "cpu_usage", 20).with_suffix("%"));
        diagnostics.add(Diagnostic::new(Self::MEM_USAGE, "mem_usage", 20).with_suffix("%"));
    }

    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        mut sysinfo: ResMut<SystemInfoResource>,
    ) {
        sysinfo.sys.refresh_cpu();
        sysinfo.sys.refresh_memory();
        let current_cpu_usage = {
            let mut usage = 0.0;
            let cpus = sysinfo.sys.cpus();
            for cpu in cpus {
                usage += cpu.cpu_usage(); // NOTE: this returns a value from 0.0 to 100.0
            }
            // average
            usage / cpus.len() as f32
        };
        // `memory()` fns return a value in bytes
        let total_mem = sysinfo.sys.total_memory() as f64 / BYTES_TO_GIB;
        let used_mem = sysinfo.sys.used_memory() as f64 / BYTES_TO_GIB;
        let current_used_mem = used_mem / total_mem * 100.0;

        diagnostics.add_measurement(Self::CPU_USAGE, || current_cpu_usage as f64);
        diagnostics.add_measurement(Self::MEM_USAGE, || current_used_mem);
    }
}
