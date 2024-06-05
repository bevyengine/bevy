use crate::DiagnosticPath;
use bevy_app::prelude::*;
use bevy_ecs::system::Resource;
use std::time::Duration;

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
    /// Total system cpu usage in %
    pub const CPU_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/cpu_usage");
    /// Total system memory usage in %
    pub const MEM_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/mem_usage");
}

/// A resource that stores diagnostic information about the system.
/// This information can be useful for debugging and profiling purposes.
///
/// # See also
///
/// [`SystemInformationDiagnosticsPlugin`] for more information.
#[derive(Debug, Resource)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub cpu: String,
    pub core_count: String,
    pub memory: String,
}

/// The expected interval at which system information will be queried and generated.
///
/// The system diagnostic plugin doesn't work in all situations. In those situations this value will
/// bet set to None.
pub const EXPECTED_SYSTEM_INFORMATION_INTERVAL: Option<Duration> =
    internal::EXPECTED_SYSTEM_INFORMATION_INTERVAL;

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
    use std::{
        sync::mpsc::{self, Receiver, Sender},
        thread,
        time::Duration,
    };

    use bevy_ecs::{prelude::ResMut, system::Local};
    use bevy_utils::tracing::info;
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

    use crate::{Diagnostic, Diagnostics, DiagnosticsStore};

    use super::{SystemInfo, SystemInformationDiagnosticsPlugin};

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    pub(crate) fn setup_system(mut diagnostics: ResMut<DiagnosticsStore>) {
        diagnostics
            .add(Diagnostic::new(SystemInformationDiagnosticsPlugin::CPU_USAGE).with_suffix("%"));
        diagnostics
            .add(Diagnostic::new(SystemInformationDiagnosticsPlugin::MEM_USAGE).with_suffix("%"));
    }

    pub(crate) const EXPECTED_SYSTEM_INFORMATION_INTERVAL: Option<Duration> =
        Some(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    /// Continuously collects diagnostic data and sends it into `diagnostic_data_sender`.
    ///
    /// This function will run in a loop until the sender closes. It should be run in
    /// another thread.
    ///
    /// The data set into the sender will be (Cpu usage %, Memory usage %)
    fn diagnostic_thread(diagnostic_data_sender: Sender<(f64, f64)>) {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(MemoryRefreshKind::everything()),
        );

        loop {
            sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
            sys.refresh_memory();
            let current_cpu_usage = sys.global_cpu_info().cpu_usage();
            // `memory()` fns return a value in bytes
            let total_mem = sys.total_memory() as f64 / BYTES_TO_GIB;
            let used_mem = sys.used_memory() as f64 / BYTES_TO_GIB;
            let current_used_mem = used_mem / total_mem * 100.0;

            if diagnostic_data_sender
                .send((current_cpu_usage.into(), current_used_mem))
                .is_err()
            {
                break;
            }

            thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    }

    pub(crate) fn diagnostic_system(
        mut diagnostics: Diagnostics,
        mut sysinfo: Local<Option<Receiver<(f64, f64)>>>,
    ) {
        let usage_receiver = sysinfo.get_or_insert_with(|| {
            let (sender, receiver) = mpsc::channel();

            // TODO: Use a builder to give the thread a name
            thread::spawn(|| diagnostic_thread(sender));

            receiver
        });

        for (current_cpu_usage, current_used_mem) in usage_receiver.try_iter() {
            diagnostics.add_measurement(&SystemInformationDiagnosticsPlugin::CPU_USAGE, || {
                current_cpu_usage
            });
            diagnostics.add_measurement(&SystemInformationDiagnosticsPlugin::MEM_USAGE, || {
                current_used_mem
            });
        }
    }

    impl Default for SystemInfo {
        fn default() -> Self {
            let sys = System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::new())
                    .with_memory(MemoryRefreshKind::new().with_ram()),
            );

            let system_info = SystemInfo {
                os: System::long_os_version().unwrap_or_else(|| String::from("not available")),
                kernel: System::kernel_version().unwrap_or_else(|| String::from("not available")),
                cpu: sys
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().trim().to_string())
                    .unwrap_or_else(|| String::from("not available")),
                core_count: sys
                    .physical_core_count()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| String::from("not available")),
                // Convert from Bytes to GibiBytes since it's probably what people expect most of the time
                memory: format!("{:.1} GiB", sys.total_memory() as f64 * BYTES_TO_GIB),
            };

            info!("{:?}", system_info);
            system_info
        }
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
    use std::time::Duration;

    pub(crate) fn setup_system() {
        bevy_utils::tracing::warn!("This platform and/or configuration is not supported!");
    }

    pub(crate) fn diagnostic_system() {
        // no-op
    }

    pub(crate) const EXPECTED_SYSTEM_INFORMATION_INTERVAL: Option<Duration> = None;

    impl Default for super::SystemInfo {
        fn default() -> Self {
            let unknown = "Unknown".to_string();
            Self {
                os: unknown.clone(),
                kernel: unknown.clone(),
                cpu: unknown.clone(),
                core_count: unknown.clone(),
                memory: unknown.clone(),
            }
        }
    }
}
