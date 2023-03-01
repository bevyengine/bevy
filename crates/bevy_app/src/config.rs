use bevy_ecs::{
    all_tuples,
    schedule::{
        BoxedScheduleLabel, Condition, IntoSystemConfig, IntoSystemSet, ScheduleLabel,
        SystemConfig, SystemConfigs, SystemSet,
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
pub trait IntoSystemAppConfig<Marker>: Sized {
    /// Converts into a [`SystemAppConfig`].
    fn into_app_config(self) -> SystemAppConfig;

    /// Adds the system to the provided `schedule`.
    ///
    /// If a schedule is not specified, it will be added to the [`App`]'s default schedule.
    ///
    /// [`App`]: crate::App
    ///
    /// # Panics
    ///
    /// If the system has already been assigned to a schedule.
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

    /// Adds the system to [`CoreSchedule::Startup`].
    /// This is a shorthand for `self.in_schedule(CoreSchedule::Startup)`.
    ///
    /// Systems in this schedule will run exactly once, at the start of the [`App`]'s lifecycle.
    ///
    /// [`App`]: crate::App
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
    ///
    /// # Panics
    ///
    /// If the system has already been assigned to a schedule.
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
    fn into_app_config(self) -> SystemAppConfig {
        self
    }
}

impl<Marker, T> IntoSystemAppConfig<Marker> for T
where
    T: IntoSystemConfig<Marker>,
{
    fn into_app_config(self) -> SystemAppConfig {
        SystemAppConfig {
            system: self.into_config(),
            schedule: None,
        }
    }
}

/// A collection of [`SystemAppConfig`]s.
pub struct SystemAppConfigs(pub(crate) InnerConfigs);

pub(crate) enum InnerConfigs {
    /// This came from an instance of `SystemConfigs`.
    /// All systems are in the same schedule.
    Blanket {
        systems: SystemConfigs,
        schedule: Option<BoxedScheduleLabel>,
    },
    /// This came from several separate instances of `SystemAppConfig`.
    /// Each system gets its own schedule.
    Granular(Vec<SystemAppConfig>),
}

/// Types that can convert into [`SystemAppConfigs`].
pub trait IntoSystemAppConfigs<Marker>: Sized {
    /// Converts to [`SystemAppConfigs`].
    fn into_app_configs(self) -> SystemAppConfigs;

    /// Adds the systems to the provided `schedule`.
    ///
    /// If a schedule is not specified, they will be added to the [`App`]'s default schedule.
    ///
    /// [`App`]: crate::App
    ///
    /// # Panics
    ///
    /// If any of the systems have already been assigned to a schedule.
    #[track_caller]
    fn in_schedule(self, label: impl ScheduleLabel) -> SystemAppConfigs {
        let mut configs = self.into_app_configs();

        match &mut configs.0 {
            InnerConfigs::Blanket { schedule, .. } => {
                if schedule.is_some() {
                    panic!(
                        "Cannot add systems to the schedule '{label:?}: they are already in '{schedule:?}'"
                    );
                }
                *schedule = Some(Box::new(label));
            }
            InnerConfigs::Granular(configs) => {
                for SystemAppConfig { schedule, .. } in configs {
                    if schedule.is_some() {
                        panic!(
                            "Cannot add system to the schedule '{label:?}': it is already in '{schedule:?}'."
                        );
                    }
                    *schedule = Some(label.dyn_clone());
                }
            }
        }

        configs
    }

    /// Adds the systems to [`CoreSchedule::Startup`].
    /// This is a shorthand for `self.in_schedule(CoreSchedule::Startup)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn startup_system_a() {}
    /// # fn startup_system_b() {}
    /// # fn startup_system_c() {}
    /// #
    /// app.add_systems(
    ///     (
    ///         startup_system_a,
    ///         startup_system_b,
    ///         startup_system_c,
    ///     )
    ///         .on_startup()
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// If any of the systems have already been assigned to a schedule.
    #[track_caller]
    fn on_startup(self) -> SystemAppConfigs {
        self.in_schedule(CoreSchedule::Startup)
    }
}

impl IntoSystemAppConfigs<()> for SystemAppConfigs {
    fn into_app_configs(self) -> SystemAppConfigs {
        self
    }
}

impl IntoSystemAppConfigs<()> for SystemConfigs {
    fn into_app_configs(self) -> SystemAppConfigs {
        SystemAppConfigs(InnerConfigs::Blanket {
            systems: self,
            schedule: None,
        })
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
                SystemAppConfigs(InnerConfigs::Granular(vec![$($sys.into_app_config(),)*]))
            }
        }
    }
}

all_tuples!(impl_system_collection, 0, 15, P, S);
