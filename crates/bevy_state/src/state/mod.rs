mod computed_states;
mod freely_mutable_state;
mod resources;
mod state_set;
mod states;
mod sub_states;
mod transitions;

pub use bevy_state_macros::*;
pub use computed_states::*;
pub use freely_mutable_state::*;
pub use resources::*;
pub use state_set::*;
pub use states::*;
pub use sub_states::*;
pub use transitions::*;

#[cfg(test)]
mod tests {
    use bevy_ecs::event::EventRegistry;
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::ScheduleLabel;
    use bevy_state_macros::States;
    use bevy_state_macros::SubStates;

    use super::*;
    use crate as bevy_state;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState {
        #[default]
        A,
        B(bool),
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum TestComputedState {
        BisTrue,
        BisFalse,
    }

    impl ComputedStates for TestComputedState {
        type SourceStates = Option<SimpleState>;

        fn compute(sources: Option<SimpleState>) -> Option<Self> {
            sources.and_then(|source| match source {
                SimpleState::A => None,
                SimpleState::B(value) => Some(if value { Self::BisTrue } else { Self::BisFalse }),
            })
        }
    }

    #[test]
    fn computed_state_with_a_single_source_is_correctly_derived() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestComputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        TestComputedState::register_computed_state_systems(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisTrue
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisFalse
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());
    }

    #[derive(SubStates, PartialEq, Eq, Debug, Default, Hash, Clone)]
    #[source(SimpleState = SimpleState::B(true))]
    enum SubState {
        #[default]
        One,
        Two,
    }

    #[test]
    fn sub_state_exists_only_when_allowed_but_can_be_modified_freely() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        SubState::register_sub_state_systems(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubState>>());

        world.insert_resource(NextState::Pending(SubState::Two));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(world.resource::<State<SubState>>().0, SubState::One);

        world.insert_resource(NextState::Pending(SubState::Two));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(world.resource::<State<SubState>>().0, SubState::Two);

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert!(!world.contains_resource::<State<SubState>>());
    }

    #[derive(SubStates, PartialEq, Eq, Debug, Default, Hash, Clone)]
    #[source(TestComputedState = TestComputedState::BisTrue)]
    enum SubStateOfComputed {
        #[default]
        One,
        Two,
    }

    #[test]
    fn substate_of_computed_states_works_appropriately() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestComputedState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubStateOfComputed>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        TestComputedState::register_computed_state_systems(&mut apply_changes);
        SubStateOfComputed::register_sub_state_systems(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());

        world.insert_resource(NextState::Pending(SubStateOfComputed::Two));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<SubStateOfComputed>>().0,
            SubStateOfComputed::One
        );

        world.insert_resource(NextState::Pending(SubStateOfComputed::Two));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<SubStateOfComputed>>().0,
            SubStateOfComputed::Two
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    struct OtherState {
        a_flexible_value: &'static str,
        another_value: u8,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum ComplexComputedState {
        InAAndStrIsBobOrJane,
        InTrueBAndUsizeAbove8,
    }

    impl ComputedStates for ComplexComputedState {
        type SourceStates = (Option<SimpleState>, Option<OtherState>);

        fn compute(sources: (Option<SimpleState>, Option<OtherState>)) -> Option<Self> {
            match sources {
                (Some(simple), Some(complex)) => {
                    if simple == SimpleState::A
                        && (complex.a_flexible_value == "bob" || complex.a_flexible_value == "jane")
                    {
                        Some(ComplexComputedState::InAAndStrIsBobOrJane)
                    } else if simple == SimpleState::B(true) && complex.another_value > 8 {
                        Some(ComplexComputedState::InTrueBAndUsizeAbove8)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    #[test]
    fn complex_computed_state_gets_derived_correctly() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<OtherState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<ComplexComputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<OtherState>>();

        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);

        ComplexComputedState::register_computed_state_systems(&mut apply_changes);

        SimpleState::register_state(&mut apply_changes);
        OtherState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(
            world.resource::<State<OtherState>>().0,
            OtherState::default()
        );
        assert!(!world.contains_resource::<State<ComplexComputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());

        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "felix",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InTrueBAndUsizeAbove8
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InAAndStrIsBobOrJane
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());
    }

    #[derive(Resource, Default)]
    struct ComputedStateTransitionCounter {
        enter: usize,
        exit: usize,
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState2 {
        #[default]
        A1,
        B2,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum TestNewcomputedState {
        A1,
        B2,
        B1,
    }

    impl ComputedStates for TestNewcomputedState {
        type SourceStates = (Option<SimpleState>, Option<SimpleState2>);

        fn compute((s1, s2): (Option<SimpleState>, Option<SimpleState2>)) -> Option<Self> {
            match (s1, s2) {
                (Some(SimpleState::A), Some(SimpleState2::A1)) => Some(TestNewcomputedState::A1),
                (Some(SimpleState::B(true)), Some(SimpleState2::B2)) => {
                    Some(TestNewcomputedState::B2)
                }
                (Some(SimpleState::B(true)), _) => Some(TestNewcomputedState::B1),
                _ => None,
            }
        }
    }

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct Startup;

    #[test]
    fn computed_state_transitions_are_produced_correctly() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SimpleState2>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestNewcomputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<SimpleState2>>();
        world.init_resource::<Schedules>();

        setup_state_transitions_in_world(&mut world, Some(Startup.intern()));

        let mut schedules = world
            .get_resource_mut::<Schedules>()
            .expect("Schedules don't exist in world");
        let apply_changes = schedules
            .get_mut(StateTransition)
            .expect("State Transition Schedule Doesn't Exist");

        TestNewcomputedState::register_computed_state_systems(apply_changes);

        SimpleState::register_state(apply_changes);
        SimpleState2::register_state(apply_changes);

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::A1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::A1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::B1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::B1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::B2));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::B2));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        world.init_resource::<ComputedStateTransitionCounter>();

        setup_state_transitions_in_world(&mut world, None);

        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(world.resource::<State<SimpleState2>>().0, SimpleState2::A1);
        assert!(!world.contains_resource::<State<TestNewcomputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.insert_resource(NextState::Pending(SimpleState2::B2));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::B2
        );
        assert_eq!(world.resource::<ComputedStateTransitionCounter>().enter, 1);
        assert_eq!(world.resource::<ComputedStateTransitionCounter>().exit, 0);

        world.insert_resource(NextState::Pending(SimpleState2::A1));
        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::A1
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            2,
            "Should Only Enter Twice"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            1,
            "Should Only Exit Once"
        );

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.insert_resource(NextState::Pending(SimpleState2::B2));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::B2
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            3,
            "Should Only Enter Three Times"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            2,
            "Should Only Exit Twice"
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<TestNewcomputedState>>());
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            3,
            "Should Only Enter Three Times"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            3,
            "Should Only Exit Twice"
        );
    }

    #[derive(Resource, Default, PartialEq, Debug)]
    struct TransitionCounter {
        exit: u8,
        transition: u8,
        enter: u8,
    }

    #[test]
    fn same_state_transition_should_emit_event_and_not_run_schedules() {
        let mut world = World::new();
        setup_state_transitions_in_world(&mut world, None);
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = world.resource_mut::<Schedules>();
        let apply_changes = schedules.get_mut(StateTransition).unwrap();
        SimpleState::register_state(apply_changes);

        let mut on_exit = Schedule::new(OnExit(SimpleState::A));
        on_exit.add_systems(|mut c: ResMut<TransitionCounter>| c.exit += 1);
        schedules.insert(on_exit);
        let mut on_transition = Schedule::new(OnTransition {
            exited: SimpleState::A,
            entered: SimpleState::A,
        });
        on_transition.add_systems(|mut c: ResMut<TransitionCounter>| c.transition += 1);
        schedules.insert(on_transition);
        let mut on_enter = Schedule::new(OnEnter(SimpleState::A));
        on_enter.add_systems(|mut c: ResMut<TransitionCounter>| c.enter += 1);
        schedules.insert(on_enter);
        world.insert_resource(TransitionCounter::default());

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(world
            .resource::<Events<StateTransitionEvent<SimpleState>>>()
            .is_empty());

        world.insert_resource(TransitionCounter::default());
        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(
            *world.resource::<TransitionCounter>(),
            TransitionCounter {
                exit: 0,
                transition: 1, // Same state transitions are allowed
                enter: 0
            }
        );
        assert_eq!(
            world
                .resource::<Events<StateTransitionEvent<SimpleState>>>()
                .len(),
            1
        );
    }

    #[test]
    fn same_state_transition_should_propagate_to_sub_state() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubState>>(&mut world);
        world.insert_resource(State(SimpleState::B(true)));
        world.init_resource::<State<SubState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        SimpleState::register_state(&mut apply_changes);
        SubState::register_sub_state_systems(&mut apply_changes);
        schedules.insert(apply_changes);
        world.insert_resource(schedules);
        setup_state_transitions_in_world(&mut world, None);

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world
                .resource::<Events<StateTransitionEvent<SimpleState>>>()
                .len(),
            1
        );
        assert_eq!(
            world
                .resource::<Events<StateTransitionEvent<SubState>>>()
                .len(),
            1
        );
    }

    #[test]
    fn same_state_transition_should_propagate_to_computed_state() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestComputedState>>(&mut world);
        world.insert_resource(State(SimpleState::B(true)));
        world.insert_resource(State(TestComputedState::BisTrue));
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        SimpleState::register_state(&mut apply_changes);
        TestComputedState::register_computed_state_systems(&mut apply_changes);
        schedules.insert(apply_changes);
        world.insert_resource(schedules);
        setup_state_transitions_in_world(&mut world, None);

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world
                .resource::<Events<StateTransitionEvent<SimpleState>>>()
                .len(),
            1
        );
        assert_eq!(
            world
                .resource::<Events<StateTransitionEvent<TestComputedState>>>()
                .len(),
            1
        );
    }

    #[derive(Resource, Default, Debug)]
    struct TransitionTracker(Vec<&'static str>);

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum TransitionTestingComputedState {
        IsA,
        IsBAndEven,
        IsBAndOdd,
    }

    impl ComputedStates for TransitionTestingComputedState {
        type SourceStates = (Option<SimpleState>, Option<SubState>);

        fn compute(sources: (Option<SimpleState>, Option<SubState>)) -> Option<Self> {
            match sources {
                (Some(simple), sub) => {
                    if simple == SimpleState::A {
                        Some(Self::IsA)
                    } else if sub == Some(SubState::One) {
                        Some(Self::IsBAndOdd)
                    } else if sub == Some(SubState::Two) {
                        Some(Self::IsBAndEven)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    #[test]
    fn check_transition_orders() {
        let mut world = World::new();
        setup_state_transitions_in_world(&mut world, None);
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TransitionTestingComputedState>>(
            &mut world,
        );
        world.insert_resource(State(SimpleState::B(true)));
        world.init_resource::<State<SubState>>();
        world.insert_resource(State(TransitionTestingComputedState::IsA));
        let mut schedules = world.remove_resource::<Schedules>().unwrap();
        let apply_changes = schedules.get_mut(StateTransition).unwrap();
        SimpleState::register_state(apply_changes);
        SubState::register_sub_state_systems(apply_changes);
        TransitionTestingComputedState::register_computed_state_systems(apply_changes);

        world.init_resource::<TransitionTracker>();
        fn register_transition(string: &'static str) -> impl Fn(ResMut<TransitionTracker>) {
            move |mut transitions: ResMut<TransitionTracker>| transitions.0.push(string)
        }

        schedules.add_systems(
            StateTransition,
            register_transition("simple exit").in_set(ExitSchedules::<SimpleState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("simple transition")
                .in_set(TransitionSchedules::<SimpleState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("simple enter").in_set(EnterSchedules::<SimpleState>::default()),
        );

        schedules.add_systems(
            StateTransition,
            register_transition("sub exit").in_set(ExitSchedules::<SubState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("sub transition")
                .in_set(TransitionSchedules::<SubState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("sub enter").in_set(EnterSchedules::<SubState>::default()),
        );

        schedules.add_systems(
            StateTransition,
            register_transition("computed exit")
                .in_set(ExitSchedules::<TransitionTestingComputedState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("computed transition")
                .in_set(TransitionSchedules::<TransitionTestingComputedState>::default()),
        );
        schedules.add_systems(
            StateTransition,
            register_transition("computed enter")
                .in_set(EnterSchedules::<TransitionTestingComputedState>::default()),
        );

        world.insert_resource(schedules);

        world.run_schedule(StateTransition);

        let transitions = &world.resource::<TransitionTracker>().0;

        assert_eq!(transitions.len(), 9);
        assert_eq!(transitions[0], "computed exit");
        assert_eq!(transitions[1], "sub exit");
        assert_eq!(transitions[2], "simple exit");
        // Transition order is arbitrary and doesn't need testing.
        assert_eq!(transitions[6], "simple enter");
        assert_eq!(transitions[7], "sub enter");
        assert_eq!(transitions[8], "computed enter");
    }
}
