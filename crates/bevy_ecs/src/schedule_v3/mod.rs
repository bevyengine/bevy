mod condition;
mod config;
mod executor;
mod graph;
mod migration;
mod schedule;
mod set;
mod state;

pub use self::condition::*;
pub use self::config::*;
pub use self::executor::*;
use self::graph::*;
pub use self::migration::*;
pub use self::schedule::*;
pub use self::set::*;
pub use self::state::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_ecs;
    use crate::system::*;
    use crate::world::World;

    #[allow(dead_code)]
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    enum TestSchedule {
        A,
        B,
        C,
        D,
        X,
    }

    #[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
    enum TestSet {
        A,
        B,
        C,
        D,
        X,
    }

    #[test]
    fn correct_order() {
        #[derive(Resource)]
        struct X(Vec<TestSet>);

        let mut world = World::new();
        world.insert_resource(X(Vec::new()));

        fn run(set: TestSet) -> impl FnMut(ResMut<X>) {
            move |mut x: ResMut<X>| {
                x.0.push(set.clone());
            }
        }

        let mut schedule = Schedule::new();
        schedule.add_systems(
            (
                run(TestSet::A),
                run(TestSet::B),
                run(TestSet::C),
                run(TestSet::D),
            )
                .chain(),
        );

        schedule.run(&mut world);
        let X(results) = world.remove_resource::<X>().unwrap();
        assert_eq!(
            results,
            vec![TestSet::A, TestSet::B, TestSet::C, TestSet::D]
        );
    }

    #[test]
    #[should_panic]
    fn dependency_loop() {
        let mut schedule = Schedule::new();
        schedule.configure_set(TestSet::X.after(TestSet::X));
    }

    #[test]
    fn dependency_cycle() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.configure_set(TestSet::A.after(TestSet::B));
        schedule.configure_set(TestSet::B.after(TestSet::A));

        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::DependencyCycle)));

        fn foo() {}
        fn bar() {}

        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_systems((foo.after(bar), bar.after(foo)));
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::DependencyCycle)));
    }

    #[test]
    #[should_panic]
    fn hierarchy_loop() {
        let mut schedule = Schedule::new();
        schedule.configure_set(TestSet::X.in_set(TestSet::X));
    }

    #[test]
    fn hierarchy_cycle() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.configure_set(TestSet::A.in_set(TestSet::B));
        schedule.configure_set(TestSet::B.in_set(TestSet::A));

        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::HierarchyCycle)));
    }

    #[test]
    fn system_type_set_ambiguity() {
        // Define some systems.
        fn foo() {}
        fn bar() {}

        let mut world = World::new();
        let mut schedule = Schedule::new();

        // Schedule `bar` to run after `foo`.
        schedule.add_system(foo);
        schedule.add_system(bar.after(foo));

        // There's only one `foo`, so it's fine.
        let result = schedule.initialize(&mut world);
        assert!(result.is_ok());

        // Schedule another `foo`.
        schedule.add_system(foo);

        // When there are multiple instances of `foo`, dependencies on
        // `foo` are no longer allowed. Too much ambiguity.
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::SystemTypeSetAmbiguity(_))));

        // same goes for `ambiguous_with`
        let mut schedule = Schedule::new();
        schedule.add_system(foo);
        schedule.add_system(bar.ambiguous_with(foo));
        let result = schedule.initialize(&mut world);
        assert!(result.is_ok());
        schedule.add_system(foo);
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::SystemTypeSetAmbiguity(_))));
    }

    #[test]
    #[should_panic]
    fn in_system_type_set() {
        fn foo() {}
        fn bar() {}

        let mut schedule = Schedule::new();
        schedule.add_system(foo.in_set(bar.into_system_set()));
    }

    #[test]
    #[should_panic]
    fn configure_system_type_set() {
        fn foo() {}
        let mut schedule = Schedule::new();
        schedule.configure_set(foo.into_system_set());
    }

    #[test]
    fn hierarchy_conflict() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        // Add `A`.
        schedule.configure_set(TestSet::A);

        // Add `B` as child of `A`.
        schedule.configure_set(TestSet::B.in_set(TestSet::A));

        // Add `X` as child of both `A` and `B`.
        schedule.configure_set(TestSet::X.in_set(TestSet::A).in_set(TestSet::B));

        // `X` cannot be the `A`'s child and grandchild at the same time.
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::HierarchyConflict)));
    }

    #[test]
    fn cross_dependency() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        // Add `B` and give it both kinds of relationships with `A`.
        schedule.configure_set(TestSet::B.in_set(TestSet::A));
        schedule.configure_set(TestSet::B.after(TestSet::A));
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::CrossDependency(_, _))));
    }

    #[test]
    fn ambiguity() {
        #[derive(Resource)]
        struct X;

        fn res_ref(_x: Res<X>) {}
        fn res_mut(_x: ResMut<X>) {}

        let mut world = World::new();
        let mut schedule = Schedule::new();

        schedule.add_systems((res_ref, res_mut));
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(BuildError::Ambiguity)));
    }

    #[test]
    fn schedule_already_exists() {
        let mut schedules = Schedules::new();
        let result = schedules.insert(TestSchedule::X, Schedule::new());
        assert!(matches!(result, Ok(())));

        let result = schedules.insert(TestSchedule::X, Schedule::new());
        assert!(matches!(result, Err(InsertionError::AlreadyExists(_))));
    }
}
