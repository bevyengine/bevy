#[cfg(all(
    any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "macos"
    ),
    not(feature = "bevy_dynamic_plugin")
))]
mod internal {
    use crate::{Diagnostic, DiagnosticId, Diagnostics};
    use bevy_app::{App, Plugin};
    use bevy_ecs::prelude::{ResMut, Resource};
    use sysinfo::{CpuExt, System, SystemExt};

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    #[derive(Resource)]
    pub struct SystemInfo {
        pub sys: System,
    }

    impl Default for SystemInfo {
        fn default() -> Self {
            SystemInfo {
                sys: System::new_all(),
            }
        }
    }

    /// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
    #[derive(Default)]
    pub struct SystemInformationDiagnosticsPlugin;

    impl Plugin for SystemInformationDiagnosticsPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(SystemInfo::default());
            app.add_startup_system(Self::setup_system)
                .add_system(Self::diagnostic_system);
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
            mut sysinfo: ResMut<SystemInfo>,
        ) {
            sysinfo.sys.refresh_cpu();
            sysinfo.sys.refresh_memory();
            let current_cpu_usage = {
                let mut usage = 0.0;
                let cpus = sysinfo.sys.cpus();
                for cpu in cpus {
                    usage += cpu.cpu_usage(); // note for future maintainers: this returns a value from 0.0 to 100.0
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
mod internal {
    use crate::{DiagnosticId, Diagnostics};
    use bevy_app::{App, Plugin};
    use bevy_ecs::prelude::ResMut;
    use bevy_log::warn;

    /// Adds a System Information Diagnostic, specifically `cpu_usage` (in %) and `mem_usage` (in %)
    #[derive(Default)]
    pub struct SystemInformationDiagnosticsPlugin;

    impl Plugin for SystemInformationDiagnosticsPlugin {
        fn build(&self, app: &mut App) {
            app.add_startup_system(Self::setup_system)
                .add_system(Self::diagnostic_system);
        }
    }

    impl SystemInformationDiagnosticsPlugin {
        pub const CPU_USAGE: DiagnosticId =
            DiagnosticId::from_u128(78494871623549551581510633532637320956);
        pub const MEM_USAGE: DiagnosticId =
            DiagnosticId::from_u128(42846254859293759601295317811892519825);

        pub fn setup_system(_diagnostics: ResMut<Diagnostics>) {
            warn!("SystemInformationDiagnosticsPlugin: This platform and/or configuration is not supported!")
        }
    }
}

pub use internal::*;
