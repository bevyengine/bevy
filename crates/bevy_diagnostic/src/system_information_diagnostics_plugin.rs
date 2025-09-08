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
/// * linux
/// * windows
/// * android
/// * macOS
///
/// NOT supported when using the `bevy/dynamic` feature even when using previously mentioned targets.
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
mod internal {
    use core::{
        pin::Pin,
        task::{Context, Poll},
    };
    use std::sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    };

    use alloc::{
        format,
        string::{String, ToString},
    };
    use atomic_waker::AtomicWaker;
    use bevy_app::{App, First, Startup, Update};
    use bevy_ecs::resource::Resource;
    use bevy_ecs::{prelude::ResMut, system::Commands};
    use bevy_platform::{cell::SyncCell, time::Instant};
    use bevy_tasks::{AsyncComputeTaskPool, Task};
    use log::info;
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

    use crate::{Diagnostic, Diagnostics, DiagnosticsStore};

    use super::{SystemInfo, SystemInformationDiagnosticsPlugin};

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    /// Sets up the system information diagnostics plugin.
    ///
    /// The plugin spawns a single background task in the async task pool that always reschedules.
    /// The [`wake_diagnostic_task`] system wakes this task once per frame during the [`First`]
    /// schedule. If enough time has passed since the last refresh, it sends [`SysinfoRefreshData`]
    /// through a channel. The [`read_diagnostic_task`] system receives this data during the
    /// [`Update`] schedule and adds it as diagnostic measurements.
    pub(super) fn setup_plugin(app: &mut App) {
        app.add_systems(Startup, setup_system)
            .add_systems(First, wake_diagnostic_task)
            .add_systems(Update, read_diagnostic_task);
    }

    fn setup_system(mut diagnostics: ResMut<DiagnosticsStore>, mut commands: Commands) {
        let (tx, rx) = mpsc::channel();
        let diagnostic_task = DiagnosticTask::new(tx);
        let waker = Arc::clone(&diagnostic_task.waker);
        let task = AsyncComputeTaskPool::get().spawn(diagnostic_task);
        commands.insert_resource(SysinfoTask {
            _task: task,
            receiver: SyncCell::new(rx),
            waker,
        });

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

    impl SysinfoRefreshData {
        fn new(system: &mut System) -> Self {
            let pid = sysinfo::get_current_pid().expect("Failed to get current process ID");
            system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);

            system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
            system.refresh_memory();

            let system_cpu_usage = system.global_cpu_usage().into();
            let total_mem = system.total_memory() as f64;
            let used_mem = system.used_memory() as f64;
            let system_mem_usage = used_mem / total_mem * 100.0;

            let process_mem_usage = system
                .process(pid)
                .map(|p| p.memory() as f64 * BYTES_TO_GIB)
                .unwrap_or(0.0);

            let process_cpu_usage = system
                .process(pid)
                .map(|p| p.cpu_usage() as f64 / system.cpus().len() as f64)
                .unwrap_or(0.0);

            Self {
                system_cpu_usage,
                system_mem_usage,
                process_cpu_usage,
                process_mem_usage,
            }
        }
    }

    #[derive(Resource)]
    struct SysinfoTask {
        _task: Task<()>,
        receiver: SyncCell<Receiver<SysinfoRefreshData>>,
        waker: Arc<AtomicWaker>,
    }

    struct DiagnosticTask {
        system: System,
        last_refresh: Instant,
        sender: Sender<SysinfoRefreshData>,
        waker: Arc<AtomicWaker>,
    }

    impl DiagnosticTask {
        fn new(sender: Sender<SysinfoRefreshData>) -> Self {
            Self {
                system: System::new_with_specifics(
                    RefreshKind::nothing()
                        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                        .with_memory(MemoryRefreshKind::everything()),
                ),
                // Avoids initial delay on first refresh
                last_refresh: Instant::now() - sysinfo::MINIMUM_CPU_UPDATE_INTERVAL,
                sender,
                waker: Arc::default(),
            }
        }
    }

    impl Future for DiagnosticTask {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.waker.register(cx.waker());

            if self.last_refresh.elapsed() > sysinfo::MINIMUM_CPU_UPDATE_INTERVAL {
                self.last_refresh = Instant::now();

                let sysinfo_refresh_data = SysinfoRefreshData::new(&mut self.system);
                self.sender.send(sysinfo_refresh_data).unwrap();
            }

            // Always reschedules
            Poll::Pending
        }
    }

    fn wake_diagnostic_task(task: ResMut<SysinfoTask>) {
        task.waker.wake();
    }

    fn read_diagnostic_task(mut diagnostics: Diagnostics, mut task: ResMut<SysinfoTask>) {
        while let Ok(data) = task.receiver.get().try_recv() {
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
mod internal {
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
