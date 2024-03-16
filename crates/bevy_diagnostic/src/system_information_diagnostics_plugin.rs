use crate::DiagnosticPath;
use bevy_app::prelude::*;

/// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
///
/// Supported targets:
/// * linux,
/// * windows,
/// * android,
/// * macos
///
/// NOT supported when using the `bevy/dynamic` feature even when using previously mentioned targets
///
/// # See also
///
/// [`LogDiagnosticsPlugin`](crate::LogDiagnosticsPlugin) to output diagnostics to the console.
#[derive(Default)]
pub struct SystemInformationDiagnosticsPlugin;
impl Plugin for SystemInformationDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, internal::setup_system)
            .add_systems(Update, internal::diagnostic_system);
    }
}

impl SystemInformationDiagnosticsPlugin {
    pub const CPU_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/cpu_usage");
    pub const MEM_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/mem_usage");
}

// NOTE: sysinfo fails to compile when using bevy dynamic or on iOS and does nothing on wasm
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "dynamic_linking")
))]
pub mod internal {
    use bevy_ecs::{prelude::ResMut, system::Local};
    use bevy_log::info;
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

    use crate::{Diagnostic, Diagnostics, DiagnosticsStore};

    use super::SystemInformationDiagnosticsPlugin;

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    pub(crate) fn setup_system(mut diagnostics: ResMut<DiagnosticsStore>) {
        diagnostics
            .add(Diagnostic::new(SystemInformationDiagnosticsPlugin::CPU_USAGE).with_suffix("%"));
        diagnostics
            .add(Diagnostic::new(SystemInformationDiagnosticsPlugin::MEM_USAGE).with_suffix("%"));
    }

    pub(crate) fn diagnostic_system(
        mut diagnostics: Diagnostics,
        mut sysinfo: Local<Option<System>>,
    ) {
        if sysinfo.is_none() {
            *sysinfo = Some(System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                    .with_memory(MemoryRefreshKind::everything()),
            ));
        }
        let Some(sys) = sysinfo.as_mut() else {
            return;
        };

        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
        sys.refresh_memory();
        let current_cpu_usage = sys.global_cpu_info().cpu_usage();
        // `memory()` fns return a value in bytes
        let total_mem = sys.total_memory() as f64 / BYTES_TO_GIB;
        let used_mem = sys.used_memory() as f64 / BYTES_TO_GIB;
        let current_used_mem = used_mem / total_mem * 100.0;

        diagnostics.add_measurement(&SystemInformationDiagnosticsPlugin::CPU_USAGE, || {
            current_cpu_usage as f64
        });
        diagnostics.add_measurement(&SystemInformationDiagnosticsPlugin::MEM_USAGE, || {
            current_used_mem
        });
    }

    #[derive(Debug)]
    // This is required because the Debug trait doesn't detect it's used when it's only used in a print :(
    #[allow(dead_code)]
    struct SystemInfo {
        os: String,
        kernel: String,
        cpu: String,
        core_count: String,
        memory: String,
    }

    pub(crate) fn log_system_info() {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu();
        sys.refresh_memory();

        let info = SystemInfo {
            os: System::long_os_version().unwrap_or_else(|| String::from("not available")),
            kernel: System::kernel_version().unwrap_or_else(|| String::from("not available")),
            cpu: sys.global_cpu_info().brand().trim().to_string(),
            core_count: sys
                .physical_core_count()
                .map(|x| x.to_string())
                .unwrap_or_else(|| String::from("not available")),
            // Convert from Bytes to GibiBytes since it's probably what people expect most of the time
            memory: format!("{:.1} GiB", sys.total_memory() as f64 * BYTES_TO_GIB),
        };

        info!("{:?}", info);
    }
}

#[cfg(not(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "dynamic_linking")
)))]
pub mod internal {
    pub(crate) fn setup_system() {
        bevy_log::warn!("This platform and/or configuration is not supported!");
    }

    pub(crate) fn diagnostic_system() {
        // no-op
    }

    pub(crate) fn log_system_info() {
        // no-op
    }
}
