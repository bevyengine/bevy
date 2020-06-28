mod diagnostic;
mod frame_time_diagnostics_plugin;
mod print_diagnostics_plugin;
#[cfg(feature = "profiler")]
mod system_profiler;
pub use diagnostic::*;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use print_diagnostics_plugin::PrintDiagnosticsPlugin;

use bevy_app::{AppBuilder, AppPlugin};

pub struct PrintDiagnostics {}

#[derive(Default)]
pub struct DiagnosticsPlugin;

impl AppPlugin for DiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Diagnostics>();
        #[cfg(feature = "profiler")]
        {
            use legion::prelude::IntoSystem;
            app.add_resource::<Box<dyn legion::systems::profiler::Profiler>>(Box::new(system_profiler::SystemProfiler::default()))
                .add_system_to_stage(bevy_app::stage::LAST, system_profiler::profiler_diagnostic_system.system());
        }
    }
}
