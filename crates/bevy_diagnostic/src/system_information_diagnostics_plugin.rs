use crate::DiagnosticPath;
use alloc::string::String;
use bevy_app::prelude::*;
use bevy_ecs::resource::Resource;

/// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
///
/// Note that gathering system information is a time intensive task and therefore can't be done on every frame.
/// Any system diagnostics gathered by this plugin may not be current when you access them.
///
/// Supported targets:
/// * linux,
/// * windows,
/// * android,
/// * macOS
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
        internal::setup_plugin(app);
    }
}

impl SystemInformationDiagnosticsPlugin {
    /// Total system cpu usage in %
    pub const SYSTEM_CPU_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/cpu_usage");
    /// Total system memory usage in %
    pub const SYSTEM_MEM_USAGE: DiagnosticPath = DiagnosticPath::const_new("system/mem_usage");
    /// Process cpu usage in %
    pub const PROCESS_CPU_USAGE: DiagnosticPath = DiagnosticPath::const_new("process/cpu_usage");
    /// Process memory usage in %
    pub const PROCESS_MEM_USAGE: DiagnosticPath = DiagnosticPath::const_new("process/mem_usage");
}

/// A resource that stores diagnostic information about the system.
/// This information can be useful for debugging and profiling purposes.
///
/// # See also
///
/// [`SystemInformationDiagnosticsPlugin`] for more information.
#[derive(Debug, Resource)]
pub struct SystemInfo {
    /// OS name and version.
    pub os: String,
    /// System kernel version.
    pub kernel: String,
    /// CPU model name.
    pub cpu: String,
    /// Physical core count.
    pub core_count: String,
    /// System RAM.
    pub memory: String,
}

// NOTE: sysinfo fails to compile when using bevy dynamic or on iOS and does nothing on Wasm
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "dynamic_linking"),
    feature = "std",
))]
pub mod internal {
    use alloc::{
        format,
        string::{String, ToString},
        sync::Arc,
    };
    use bevy_app::{App, First, Startup, Update};
    use bevy_ecs::{prelude::ResMut, resource::Resource};
    use bevy_platform::time::Instant;
    use bevy_tasks::AsyncComputeTaskPool;
    use core::{
        pin::Pin,
        task::{Context, Poll},
    };
    use crossbeam_channel::{Receiver, Sender};
    use log::info;
    use std::sync::Mutex;
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

    use crate::{Diagnostic, Diagnostics, DiagnosticsStore};

    use super::{SystemInfo, SystemInformationDiagnosticsPlugin};

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    pub(super) fn setup_plugin(app: &mut App) {
        app.add_systems(Startup, setup_system)
            .add_systems(First, launch_diagnostic_job)
            .add_systems(Update, read_diagnostic_tasks)
            .init_resource::<DiagnosticJobData>();
    }

    fn setup_system(mut diagnostics: ResMut<DiagnosticsStore>) {
        diagnostics.add(
            Diagnostic::new(SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE).with_suffix("%"),
        );
        diagnostics.add(
            Diagnostic::new(SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE).with_suffix("%"),
        );
        diagnostics.add(
            Diagnostic::new(SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE).with_suffix("%"),
        );
        diagnostics.add(
            Diagnostic::new(SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE)
                .with_suffix("GiB"),
        );
    }

    struct SysinfoRefreshData {
        system_cpu_usage: f64,
        system_mem_usage: f64,
        process_cpu_usage: f64,
        process_mem_usage: f64,
    }

    struct DiagnosticJob {
        last_refresh: Option<Instant>,
        data_rx: Sender<SysinfoRefreshData>,
        sysinfo: Arc<Mutex<System>>,
    }

    #[derive(Default, Resource)]
    struct DiagnosticJobData {
        data_rx: Option<Receiver<SysinfoRefreshData>>,
    }

    impl Future for DiagnosticJob {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            cx.waker().wake_by_ref();

            let run_diagnostic = self
                .last_refresh
                .map(|last_refresh| last_refresh.elapsed() > sysinfo::MINIMUM_CPU_UPDATE_INTERVAL)
                .unwrap_or(true);

            if run_diagnostic {
                {
                    let mut sys = self.sysinfo.lock().unwrap();
                    let data = get_diagnostic_data(&mut sys);
                    _ = self.data_rx.send(data);
                }
                self.get_mut().last_refresh = Some(Instant::now());
            }

            Poll::Pending
        }
    }

    fn launch_diagnostic_job(mut data: ResMut<DiagnosticJobData>) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        data.data_rx = Some(rx);
        AsyncComputeTaskPool::get()
            .spawn(DiagnosticJob {
                last_refresh: None,
                data_rx: tx,
                sysinfo: Arc::new(Mutex::new(System::new_with_specifics(
                    RefreshKind::nothing()
                        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                        .with_memory(MemoryRefreshKind::everything()),
                ))),
            })
            .detach();
    }

    fn get_diagnostic_data(sys: &mut System) -> SysinfoRefreshData {
        let pid = sysinfo::get_current_pid().expect("Failed to get current process ID");

        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        sys.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
        sys.refresh_memory();

        let system_cpu_usage = sys.global_cpu_usage().into();

        let system_mem_usage = sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0;

        let process_mem_usage = sys
            .process(pid)
            .map(|p| p.memory() as f64 * BYTES_TO_GIB)
            .unwrap_or(0.0);

        let process_cpu_usage = sys
            .process(pid)
            .map(|p| p.cpu_usage() as f64 / sys.cpus().len() as f64)
            .unwrap_or(0.0);

        SysinfoRefreshData {
            system_cpu_usage,
            system_mem_usage,
            process_cpu_usage,
            process_mem_usage,
        }
    }

    fn read_diagnostic_tasks(mut diagnostics: Diagnostics, mut data: ResMut<DiagnosticJobData>) {
        if let Some(data_rx) = data.data_rx.as_mut() {
            let Ok(data) = data_rx.try_recv() else {
                return;
            };

            diagnostics.add_measurement(
                &SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE,
                || data.system_cpu_usage,
            );
            diagnostics.add_measurement(
                &SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE,
                || data.system_mem_usage,
            );
            diagnostics.add_measurement(
                &SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
                || data.process_cpu_usage,
            );
            diagnostics.add_measurement(
                &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
                || data.process_mem_usage,
            );
        }
    }

    impl Default for SystemInfo {
        fn default() -> Self {
            let sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing())
                    .with_memory(MemoryRefreshKind::nothing().with_ram()),
            );

            let system_info = SystemInfo {
                os: System::long_os_version().unwrap_or_else(|| String::from("not available")),
                kernel: System::kernel_version().unwrap_or_else(|| String::from("not available")),
                cpu: sys
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().trim().to_string())
                    .unwrap_or_else(|| String::from("not available")),
                core_count: System::physical_core_count()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| String::from("not available")),
                // Convert from Bytes to GibiBytes since it's probably what people expect most of the time
                memory: format!("{:.1} GiB", sys.total_memory() as f64 * BYTES_TO_GIB),
            };

            info!("{system_info:?}");
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
    not(feature = "dynamic_linking"),
    feature = "std",
)))]
pub mod internal {
    use alloc::string::ToString;
    use bevy_app::{App, Startup};

    pub(super) fn setup_plugin(app: &mut App) {
        app.add_systems(Startup, setup_system);
    }

    fn setup_system() {
        log::warn!("This platform and/or configuration is not supported!");
    }

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
