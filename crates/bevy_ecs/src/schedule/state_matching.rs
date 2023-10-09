use super::{Condition, States};
use crate::{change_detection::Res, system::IntoSystem};
pub use bevy_ecs_macros::{entering, exiting, state_matches, transitioning};
use std::ops::Deref;

/// An enum describing the possible result of a state transition match.
///
/// If you are just matching a single state, treat `TransitionMatches` and `MainMatches` as truthy
/// If you are matching a transition between two states, only `TransitionMatches` should be considered truthy
#[derive(Eq, Clone, Copy, PartialEq, Debug)]
pub enum MatchesStateTransition {
    /// This means the transition is considered valid by the matcher.
    TransitionMatches,
    /// This means that the Main value matches, but the transition as a whole might not. Useful for inferring the `match_state` function in a matcher, handling `every` macros.
    MainMatches,
    /// This means that neither the Main value doesn't match, and the transition is invalid.
    NoMatch,
}

impl From<bool> for MatchesStateTransition {
    fn from(value: bool) -> Self {
        match value {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::NoMatch,
        }
    }
}
mod sealed {
    use std::marker::PhantomData;

    pub trait Marker {}

    pub struct IsSingleStateMatcher<M>(PhantomData<M>);

    impl<M> Marker for IsSingleStateMatcher<M> {}
    pub struct IsTransitionMatcher<M>(PhantomData<M>);

    impl<M> Marker for IsTransitionMatcher<M> {}

    pub struct IsState;
    impl Marker for IsState {}

    pub struct StateRef;
    impl Marker for StateRef {}

    pub struct OptStateRef;
    impl Marker for OptStateRef {}

    impl<M1: Marker, M2: Marker> Marker for (M1, M2) {}

    pub struct BoolReturn;
    impl Marker for BoolReturn {}

    pub struct TransitionReturn;
    impl Marker for TransitionReturn {}

    pub struct IsFn<In: Marker, Out: Marker>(PhantomData<(In, Out)>);
    impl<In: Marker, Out: Marker> Marker for IsFn<In, Out> {}

    pub trait StateMatcher<S: super::States, Marker> {}
}
/// Types that can match world-wide states.
pub trait StateMatcher<S: States, Marker>:
    Send + Sync + Sized + 'static + sealed::StateMatcher<S, Marker>
{
    /// Check whether to match with the current state
    fn match_state(&self, state: &S) -> bool;

    /// Check whether to match a state transition
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition;
}

/// Define a state matcher using a single state conditional
pub trait SingleStateMatcher<S: States, Marker: sealed::Marker>:
    Send + Sync + Sized + 'static
{
    /// Check whether to match with the current state
    fn match_single_state(&self, state: &S) -> bool;
}

/// Define a state matcher with custom transition logic
pub trait TransitionStateMatcher<S: States, Marker: sealed::Marker>:
    Send + Sync + Sized + 'static
{
    /// Check whether to match a state transition
    fn match_transition(&self, main: Option<&S>, secondary: Option<&S>) -> MatchesStateTransition;
}

impl<S: States, M: sealed::Marker, Matcher: SingleStateMatcher<S, M>>
    sealed::StateMatcher<S, sealed::IsSingleStateMatcher<M>> for Matcher
{
}

impl<S: States, M: sealed::Marker, Matcher: SingleStateMatcher<S, M>>
    StateMatcher<S, sealed::IsSingleStateMatcher<M>> for Matcher
{
    fn match_state(&self, state: &S) -> bool {
        self.match_single_state(state)
    }

    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        match main.map(|s| self.match_single_state(s)) {
            Some(true) => match secondary {
                Some(s) => match self.match_single_state(s) {
                    true => MatchesStateTransition::MainMatches,
                    false => MatchesStateTransition::TransitionMatches,
                },
                None => MatchesStateTransition::TransitionMatches,
            },
            _ => MatchesStateTransition::NoMatch,
        }
    }
}

impl<S: States, M: sealed::Marker, Matcher: TransitionStateMatcher<S, M>>
    sealed::StateMatcher<S, sealed::IsTransitionMatcher<M>> for Matcher
{
}

impl<S: States, M: sealed::Marker, Matcher: TransitionStateMatcher<S, M>>
    StateMatcher<S, sealed::IsTransitionMatcher<M>> for Matcher
{
    fn match_state(&self, state: &S) -> bool {
        self.match_transition(Some(state), None) != MatchesStateTransition::NoMatch
    }

    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        self.match_transition(main, secondary)
    }
}

impl<S: States> SingleStateMatcher<S, sealed::IsState> for S {
    fn match_single_state(&self, state: &S) -> bool {
        state == self
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S) -> bool>
    SingleStateMatcher<S, sealed::IsFn<sealed::StateRef, sealed::BoolReturn>> for F
{
    fn match_single_state(&self, state: &S) -> bool {
        self(state)
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>) -> bool>
    sealed::StateMatcher<S, sealed::IsFn<sealed::OptStateRef, sealed::BoolReturn>> for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>) -> bool>
    StateMatcher<S, sealed::IsFn<sealed::OptStateRef, sealed::BoolReturn>> for F
{
    fn match_state(&self, state: &S) -> bool {
        self(Some(state))
    }

    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        if self(main) {
            match self(secondary) {
                true => MatchesStateTransition::MainMatches,
                false => MatchesStateTransition::TransitionMatches,
            }
        } else {
            MatchesStateTransition::NoMatch
        }
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, &S) -> MatchesStateTransition>
    sealed::StateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::TransitionReturn>,
    > for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, &S) -> MatchesStateTransition>
    StateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::TransitionReturn>>
    for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        let Some(secondary) = secondary else {
            return false.into();
        };
        self(main, secondary)
    }

    fn match_state(&self, _: &S) -> bool {
        false
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> MatchesStateTransition>
    sealed::StateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::TransitionReturn>,
    > for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> MatchesStateTransition>
    StateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::TransitionReturn>>
    for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        self(main, secondary)
    }

    fn match_state(&self, state: &S) -> bool {
        self(state, None) != MatchesStateTransition::NoMatch
    }
}

impl<
        S: States,
        F: 'static + Send + Sync + Fn(Option<&S>, Option<&S>) -> MatchesStateTransition,
    >
    TransitionStateMatcher<
        S,
        sealed::IsFn<(sealed::OptStateRef, sealed::OptStateRef), sealed::TransitionReturn>,
    > for F
{
    fn match_transition(&self, main: Option<&S>, secondary: Option<&S>) -> MatchesStateTransition {
        self(main, secondary)
    }
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, &S) -> bool>
    sealed::StateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::BoolReturn>>
    for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, &S) -> bool>
    StateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::BoolReturn>> for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        let Some(secondary) = secondary else {
            return false.into();
        };
        self(main, secondary).into()
    }

    fn match_state(&self, _: &S) -> bool {
        false
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> bool>
    sealed::StateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::BoolReturn>,
    > for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> bool>
    StateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::BoolReturn>>
    for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        if !self(main, None) {
            return false.into();
        }
        match self(main, secondary) {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::MainMatches,
        }
    }

    fn match_state(&self, state: &S) -> bool {
        self(state, None)
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>, Option<&S>) -> bool>
    sealed::StateMatcher<
        S,
        sealed::IsFn<(sealed::OptStateRef, sealed::OptStateRef), sealed::BoolReturn>,
    > for F
{
}
impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>, Option<&S>) -> bool>
    StateMatcher<S, sealed::IsFn<(sealed::OptStateRef, sealed::OptStateRef), sealed::BoolReturn>>
    for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        if !self(main, None) {
            return false.into();
        }
        match self(main, secondary) {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::MainMatches,
        }
    }
    fn match_state(&self, state: &S) -> bool {
        self(Some(state), None)
    }
}

/// Get a [`Condition`] for running whenever `MainResource<S>` matches regardless of
/// whether `SecondaryResource<S>` matches, so long as they are not identical
pub(crate) fn run_condition_on_match<
    MainResource: crate::prelude::Resource + Deref<Target = S>,
    SecondaryResource: crate::prelude::Resource + Deref<Target = S>,
    S: States,
    M,
>(
    matcher: impl StateMatcher<S, M>,
) -> impl Condition<()> {
    IntoSystem::into_system(
        move |main: Option<Res<MainResource>>, secondary: Option<Res<SecondaryResource>>| {
            let main = main.as_ref().map(|v| v.as_ref().deref());
            let secondary = secondary.as_ref().map(|v| v.as_ref().deref());

            if let (Some(main), Some(secondary)) = (main, secondary) {
                if main == secondary {
                    return false;
                }
            }

            let result = matcher.match_state_transition(main, secondary);
            result == MatchesStateTransition::TransitionMatches
        },
    )
}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::{entering, state_matches};

    use crate as bevy_ecs;
    use crate::change_detection::Ticks;
    use crate::component::Tick;
    use crate::schedule::PreviousState;
    use crate::system::{IntoSystem, System};
    use crate::{
        change_detection::Res,
        schedule::{MatchesStateTransition, State, StateMatcher, States},
        world::World,
    };

    use super::{sealed, SingleStateMatcher, TransitionStateMatcher};

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum TestState {
        #[default]
        A,
        B,
        C(bool),
    }

    #[test]
    fn a_state_matches_against_itself() {
        let a = TestState::A;
        assert!(a.match_state(&a));
    }

    #[test]
    fn a_state_matches_doesnt_match_another_variant() {
        let a = TestState::A;
        assert!(!a.match_state(&TestState::B));
    }
    struct OnlyC;

    impl SingleStateMatcher<TestState, sealed::IsState> for OnlyC {
        fn match_single_state(&self, state: &TestState) -> bool {
            matches!(state, TestState::C(_))
        }
    }

    #[test]
    fn a_single_state_matcher_matches_all_relevant_variants() {
        assert!(OnlyC.match_state(&TestState::C(true)));
        assert!(OnlyC.match_state(&TestState::C(false)));
        assert!(!OnlyC.match_state(&TestState::A));
        assert!(!OnlyC.match_state(&TestState::B));
    }

    #[test]
    fn a_single_state_matcher_matches_transitions_in_and_out_of_a_match_and_not_within_it() {
        assert_eq!(
            OnlyC.match_state_transition(Some(&TestState::C(true)), Some(&TestState::A)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            OnlyC.match_state_transition(Some(&TestState::C(false)), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            OnlyC.match_state_transition(Some(&TestState::C(true)), Some(&TestState::C(false))),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            OnlyC.match_state_transition(Some(&TestState::A), Some(&TestState::C(false))),
            MatchesStateTransition::NoMatch
        );
    }

    struct AtoB;

    impl TransitionStateMatcher<TestState, sealed::IsState> for AtoB {
        fn match_transition(
            &self,
            main: Option<&TestState>,
            secondary: Option<&TestState>,
        ) -> MatchesStateTransition {
            let Some(main) = main else {
                return MatchesStateTransition::NoMatch;
            };
            if main == &TestState::A {
                match secondary {
                    Some(&TestState::B) => MatchesStateTransition::TransitionMatches,
                    _ => MatchesStateTransition::MainMatches,
                }
            } else {
                MatchesStateTransition::NoMatch
            }
        }
    }

    #[test]
    fn a_transition_state_matcher_can_match_single_states() {
        assert!(AtoB.match_state(&TestState::A));
        assert!(!AtoB.match_state(&TestState::B));
    }

    #[test]
    fn a_transition_state_matcher_can_match_state_transitions() {
        assert_eq!(
            AtoB.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            AtoB.match_state_transition(Some(&TestState::A), Some(&TestState::C(false))),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            AtoB.match_state_transition(Some(&TestState::A), None),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            AtoB.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            AtoB.match_state_transition(None, Some(&TestState::B)),
            MatchesStateTransition::NoMatch
        );
    }

    #[test]
    fn fn_auto_implementations_of_state_matcher_match_appropriately() {
        let test_func = |state: &TestState| state == &TestState::A;
        assert!(test_func.match_state(&TestState::A));
        assert!(!test_func.match_state(&TestState::B));
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::A)),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(None, Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );

        let test_func = |state: Option<&TestState>| match state {
            Some(state) => state == &TestState::A,
            None => true,
        };
        assert!(test_func.match_state(&TestState::A));
        assert!(!test_func.match_state(&TestState::B));
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::A)),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(None, Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );

        let test_func =
            |from: &TestState, to: &TestState| from == &TestState::A && to == &TestState::B;
        assert!(!test_func.match_state(&TestState::A));
        assert!(!test_func.match_state(&TestState::B));
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(None, Some(&TestState::B)),
            MatchesStateTransition::NoMatch
        );

        let test_func = |from: &TestState, to: Option<&TestState>| {
            from == &TestState::A
                && match to {
                    Some(to) => to == &TestState::B,
                    None => true,
                }
        };
        assert!(test_func.match_state(&TestState::A));
        assert!(!test_func.match_state(&TestState::B));
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::A)),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), None),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(None, Some(&TestState::B)),
            MatchesStateTransition::NoMatch
        );

        let test_func = |from: Option<&TestState>, to: Option<&TestState>| {
            (match from {
                Some(from) => from == &TestState::A,
                None => true,
            }) && (match to {
                Some(to) => to == &TestState::B,
                None => true,
            })
        };
        assert!(test_func.match_state(&TestState::A));
        assert!(!test_func.match_state(&TestState::B));
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), Some(&TestState::A)),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            test_func.match_state_transition(Some(&TestState::A), None),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            test_func.match_state_transition(None, Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
    }

    #[test]
    fn macro_can_generate_matcher_for_single_expression() {
        let state_a = State::new(TestState::A);
        let state_b = State::new(TestState::B);
        let state_c = State::new(TestState::C(true));
        let tick = Tick::new(0);

        let mut match_state_value = state_matches!(TestState::A);
        assert!(match_state_value(Some(Res {
            value: &state_a,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_b,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        let mut match_state_value = state_matches!(OnlyC);
        assert!(match_state_value(Some(Res {
            value: &state_c,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_b,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_simple_pattern() {
        let state_a = State::new(TestState::A);
        let state_b = State::new(TestState::B);
        let state_c = State::new(TestState::C(true));
        let state_c2 = State::new(TestState::C(false));
        let tick = Tick::new(0);
        let match_state_value = state_matches!(TestState, C(_));
        assert!(match_state_value(Some(Res {
            value: &state_c,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(match_state_value(Some(Res {
            value: &state_c2,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_a,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_b,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_closure() {
        let state_a = State::new(TestState::A);
        let state_b = State::new(TestState::B);
        let state_c = State::new(TestState::C(true));
        let state_c2 = State::new(TestState::C(false));
        let tick = Tick::new(0);
        let match_state_value = state_matches!(TestState, |state: &TestState| matches!(
            state,
            TestState::C(_)
        ));
        assert!(match_state_value(Some(Res {
            value: &state_c,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(match_state_value(Some(Res {
            value: &state_c2,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_a,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
        assert!(!match_state_value(Some(Res {
            value: &state_b,
            ticks: Ticks {
                added: &tick,
                changed: &tick,
                last_run: tick,
                this_run: tick,
            },
        })));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_simple_transition() {
        let mut world = World::new();

        let match_state_value = entering!(TestState, C(_));

        let mut system = IntoSystem::into_system(match_state_value);

        system.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::A));
        assert!(system.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::C(false)));
        assert!(!system.run((), &mut world));
    }
    #[test]
    fn macro_can_generate_matcher_for_every_transition() {
        let mut world = World::new();

        let match_state_value = entering!(TestState, every C(_));

        let mut system = IntoSystem::into_system(match_state_value);

        system.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::A));
        assert!(system.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::C(false)));
        assert!(system.run((), &mut world));
        world.insert_resource(PreviousState::new(TestState::C(true)));
        world.insert_resource(State::new(TestState::A));
        assert!(!system.run((), &mut world));
    }
    #[test]
    fn macro_can_generate_multi_pattern_matcher() {
        let mut world = World::new();

        let match_state_value = entering!(TestState, C(_), every |_: &TestState| true);

        let mut system = IntoSystem::into_system(match_state_value);

        system.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::A));
        assert!(system.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(PreviousState::new(TestState::C(false)));
        assert!(!system.run((), &mut world));
        world.insert_resource(PreviousState::new(TestState::C(true)));
        world.insert_resource(State::new(TestState::A));
        assert!(system.run((), &mut world));
    }
}
