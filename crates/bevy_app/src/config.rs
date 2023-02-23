use bevy_ecs::schedule::{
    BoxedScheduleLabel, Condition, IntoSystemConfig, IntoSystemSet, IntoSystemSetConfig,
    ScheduleLabel, SystemConfig, SystemSet, SystemSetConfig,
};

use crate::CoreSchedule;

/// A [`SystemSet`] with [`App`]-aware scheduling metadata.
///
/// [`App`]: crate::App
pub struct SystemSetAppConfig {
    pub(crate) config: SystemSetConfig,
    pub(crate) schedule: Option<BoxedScheduleLabel>,
}

/// Types that can be converted into a [`SystemSetAppConfig`].
pub trait IntoSystemSetAppConfig: Sized + IntoSystemSetConfig {
    /// Converts into a [`SystemSetAppConfig`].
    #[doc(hidden)]
    fn into_app_config(self) -> SystemSetAppConfig;

    /// Add to the provided `schedule`.
    #[track_caller]
    fn in_schedule(self, schedule: impl ScheduleLabel) -> SystemSetAppConfig {
        let mut config = self.into_app_config();
        if let Some(old_schedule) = &config.schedule {
            panic!(
                "Cannot add system set to schedule '{schedule:?}': it is already in '{old_schedule:?}'."
            );
        }
        config.schedule = Some(Box::new(schedule));

        config
    }
}

impl IntoSystemSetConfig for SystemSetAppConfig {
    type Config = Self;

    fn into_config(self) -> Self::Config {
        self
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.in_set(set),
            schedule,
        }
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.in_base_set(set),
            schedule,
        }
    }

    fn in_default_base_set(self) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.in_default_base_set(),
            schedule,
        }
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.before(set),
            schedule,
        }
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.after(set),
            schedule,
        }
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.run_if(condition),
            schedule,
        }
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.ambiguous_with(set),
            schedule,
        }
    }

    fn ambiguous_with_all(self) -> Self {
        let Self { config, schedule } = self;
        Self {
            config: config.ambiguous_with_all(),
            schedule,
        }
    }
}

impl<T> IntoSystemSetAppConfig for T
where
    T: IntoSystemSetConfig<Config = SystemSetConfig>,
{
    fn into_app_config(self) -> SystemSetAppConfig {
        SystemSetAppConfig {
            config: self.into_config(),
            schedule: None,
        }
    }
}

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
    fn on_startup(self) -> SystemAppConfig {
        let mut config = self.into_app_config();
        if let Some(old_schedule) = &config.schedule {
            panic!(
                "Cannot add system to the startup schedule: it is already in '{old_schedule:?}'."
            );
        }
        config.schedule = Some(Box::new(CoreSchedule::Startup));

        config
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
