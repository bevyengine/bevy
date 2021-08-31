use bevy_ecs::system::StageConfig;

use crate::StartupStage;

trait StartupConfig<Params, Configured> {
    fn startup(self) -> Configured;
}

impl<T, Params, Configured> StartupConfig<Params, Configured> for T
where
    T: StageConfig<Params, Configured>,
{
    fn startup(self) -> Configured {
        self.stage(StartupStage::Startup)
    }
}
