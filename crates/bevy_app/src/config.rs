use bevy_ecs::{
    all_tuples,
    schedule::{
        BoxedScheduleLabel, Condition, IntoSystemConfig, IntoSystemConfigs, IntoSystemSet,
        ScheduleLabel, SystemConfig, SystemConfigs, SystemSet,
    },
};

use crate::CoreSchedule;

/// A [`System`] with [`App`]-aware scheduling metadata.
///
/// [`System`]: bevy_ecs::prelude::System
/// [`App`]: crate::App
pub struct SystemAppConfig {
    pub(crate) system: SystemConfig,
    pub(crate) schedule: Option<BoxedScheduleLabel>,
}

/// Types that can be converted into a [`SystemAppConfig`].
///
/// This has been implemented for all `System<In = (), Out = ()>` trait objects
/// and all functions that convert into such.
pub trait IntoSystemAppConfig<Marker, Config = SystemAppConfig>:
    Sized + IntoSystemConfig<Marker, Self::InnerConfig>
{
    #[doc(hidden)]
    type InnerConfig;

    /// Converts into a [`SystemAppConfig`].
    fn into_app_config(self) -> SystemAppConfig;

    /// Add to the provided `schedule`.
    #[track_caller]
    fn in_schedule(self, schedule: impl ScheduleLabel) -> SystemAppConfig {
        let mut config = self.into_app_config();
        if let Some(old_schedule) = &config.schedule {
            panic!(
                "Cannot add system to schedule '{schedule:?}': it is already in '{old_schedule:?}'."
            );
        }
        config.schedule = Some(Box::new(schedule));

        config
    }

    /// Add to [`CoreSchedule::Startup`].
    ///
    /// These systems will run exactly once, at the start of the [`App`]'s lifecycle.
    /// To add a system that runs every frame, see [`add_system`](Self::add_system).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// fn my_startup_system(_commands: Commands) {
    ///     println!("My startup system");
    /// }
    ///
    /// App::new()
    ///     .add_system(my_startup_system.on_startup())
    ///     .run();
    /// ```
    #[inline]
    fn on_startup(self) -> SystemAppConfig {
        self.in_schedule(CoreSchedule::Startup)
    }
}

impl IntoSystemConfig<(), Self> for SystemAppConfig {
    fn into_config(self) -> Self {
        self
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.in_set(set),
            schedule,
        }
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.in_base_set(set),
            schedule,
        }
    }

    fn no_default_base_set(self) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.no_default_base_set(),
            schedule,
        }
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.before(set),
            schedule,
        }
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.after(set),
            schedule,
        }
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.run_if(condition),
            schedule,
        }
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.ambiguous_with(set),
            schedule,
        }
    }

    fn ambiguous_with_all(self) -> Self {
        let Self { system, schedule } = self;
        Self {
            system: system.ambiguous_with_all(),
            schedule,
        }
    }
}

impl IntoSystemAppConfig<()> for SystemAppConfig {
    type InnerConfig = Self;

    fn into_app_config(self) -> SystemAppConfig {
        self
    }
}

impl<Marker, T> IntoSystemAppConfig<Marker> for T
where
    T: IntoSystemConfig<Marker>,
{
    type InnerConfig = SystemConfig;

    fn into_app_config(self) -> SystemAppConfig {
        SystemAppConfig {
            system: self.into_config(),
            schedule: None,
        }
    }
}

/// A collection of [`SystemAppConfig`]s.
pub struct SystemAppConfigs {
    pub(crate) systems: SystemConfigs,
    pub(crate) schedule: ScheduleMode,
}

pub(crate) enum ScheduleMode {
    None,
    Blanket(BoxedScheduleLabel),
    Granular(Vec<Option<BoxedScheduleLabel>>),
}

/// Types that can convert into [`SystemAppConfigs`].
pub trait IntoSystemAppConfigs<Marker>: Sized {
    /// Converts to [`SystemAppConfigs`].
    fn into_app_configs(self) -> SystemAppConfigs;

    /// Adds the systems to the provided `schedule`.
    fn in_schedule(self, schedule: impl ScheduleLabel) -> SystemAppConfigs {
        let mut configs = self.into_app_configs();

        match &configs.schedule {
            ScheduleMode::None => {}
            ScheduleMode::Blanket(old_schedule) => panic!(
                "Cannot add systems to the schedule '{schedule:?}: they are already in '{old_schedule:?}'"
            ),
            ScheduleMode::Granular(slots) => {
                for slot in slots {
                    if let Some(old_schedule) = &slot {
                        panic!(
                            "Cannot add system to the schedule '{schedule:?}': it is already in '{old_schedule:?}'."
                        );
                    }
                }
            }
        }

        configs.schedule = ScheduleMode::Blanket(Box::new(schedule));

        configs
    }
}

impl IntoSystemAppConfigs<()> for SystemAppConfigs {
    fn into_app_configs(self) -> SystemAppConfigs {
        self
    }
}

impl IntoSystemAppConfigs<()> for SystemConfigs {
    fn into_app_configs(self) -> SystemAppConfigs {
        SystemAppConfigs {
            systems: self,
            schedule: ScheduleMode::None,
        }
    }
}

macro_rules! impl_system_collection {
    ($(($param: ident, $sys: ident)),*) => {
        impl<$($param, $sys),*> IntoSystemAppConfigs<($($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoSystemAppConfig<$param>),*
        {
            #[allow(non_snake_case)]
            fn into_app_configs(self) -> SystemAppConfigs {
                let ($($sys,)*) = self;
                $(
                    let mut $sys = $sys.into_app_config();
                )*
                SystemAppConfigs {
                    schedule: ScheduleMode::Granular(vec![$($sys.schedule.take(),)*]),
                    systems: ($($sys.system,)*).into_configs(),
                }
            }
        }
    }
}

all_tuples!(impl_system_collection, 0, 15, P, S);
