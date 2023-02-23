use bevy_ecs::schedule::{
    BoxedScheduleLabel, Condition, IntoSystemConfig, IntoSystemSet, ScheduleLabel, SystemConfig,
    SystemSet,
};

/// A [`System`] with [`App`]-aware scheduling metadata.
///
/// [`System`]: bevy_ecs::prelude::System
/// [`App`]: crate::App
pub struct SystemAppConfig {
    pub(crate) system: SystemConfig,
    pub(crate) schedule: Option<BoxedScheduleLabel>,
}

/*mod sealed {
    use bevy_ecs::schedule::IntoSystemConfig;

    #[doc(hidden)]
    pub trait IntoSystemAppConfig<Marker> {}

    impl IntoSystemAppConfig<()> for super::SystemAppConfig {}

    impl<Marker, T> IntoSystemAppConfig<Marker> for T where T: IntoSystemConfig<Marker> {}
}*/

/// Types that can be converted into a [`SystemAppConfig`].
///
/// This has been implemented for all `System<In = (), Out = ()>` trait objects
/// and all functions that convert into such.
pub trait IntoSystemAppConfig<Marker>: Sized + IntoSystemConfig<Marker> {
    /// Converts into a [`SystemAppConfig`].
    fn into_app_config(self) -> SystemAppConfig;

    /// Add to the provided `schedule`.
    #[track_caller]
    fn in_schedule(self, schedule: impl ScheduleLabel) -> SystemAppConfig {
        let mut config = self.into_app_config();
        if let Some(old_schedule) = &config.schedule {
            panic!(
                "Cannot add system to schedule '{schedule:?}': it is already in {old_schedule:?}."
            );
        }
        config.schedule = Some(Box::new(schedule));

        config
    }
}

impl IntoSystemConfig<()> for SystemAppConfig {
    type Config = Self;

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
    T: IntoSystemConfig<Marker, Config = SystemConfig>,
{
    fn into_app_config(self) -> SystemAppConfig {
        SystemAppConfig {
            system: self.into_config(),
            schedule: None,
        }
    }
}
