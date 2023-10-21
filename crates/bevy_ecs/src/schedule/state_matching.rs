use super::{ActiveTransition, State, States};
use crate::{
    archetype::ArchetypeComponentId,
    change_detection::Res,
    component::ComponentId,
    query::Access,
    system::{IntoSystem, ReadOnlySystem, System},
    world::unsafe_world_cell::UnsafeWorldCell,
};
pub use bevy_ecs_macros::state_matches;
use std::{borrow::Cow, marker::PhantomData};

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

/// A wrapper around a `StateMatcher` that ignores the state matcher's
/// `match_state_transition`, and instead always returns a
/// `TransitionMatches` if the main state matches.
pub struct EveryTransition<S: States, Sm: StateMatcher<S, Marker>, Marker: 'static>(
    pub Sm,
    PhantomData<Box<dyn Send + Sync + 'static + Fn(S) -> Marker>>,
);

impl<S: States, Marker: Send + Sync + 'static, Sm: StateMatcher<S, Marker>>
    sealed::InternalStateMatcher<S, ()> for EveryTransition<S, Sm, Marker>
{
    fn match_state(&self, state: &S) -> bool {
        self.0.match_state(state)
    }

    fn match_state_transition(&self, main: Option<&S>, _: Option<&S>) -> MatchesStateTransition {
        if let Some(main) = main {
            self.0.match_state(main).into()
        } else {
            false.into()
        }
    }
}

/// A wrapper around a `StateMatcher` that swaps the `main` and `seconary` states
/// when calling `main_state_transition`.
///
/// When `Exiting`, the `main` would become the incoming state
/// When `Entering`, the `main` would become the outgoing state
pub struct InvertTransition<S: States, Sm: StateMatcher<S, Marker>, Marker: 'static>(
    Sm,
    PhantomData<Box<dyn Send + Sync + 'static + Fn(S) -> Marker>>,
);

impl<S: States, Marker, Sm: StateMatcher<S, Marker>> sealed::InternalStateMatcher<S, ()>
    for InvertTransition<S, Sm, Marker>
{
    fn match_state(&self, state: &S) -> bool {
        self.0.match_state(state)
    }

    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        self.0.match_state_transition(secondary, main)
    }
}

/// A struct that takes two `StateMatcher`s, and evaluates them in order.
///
/// If matching a single state, it will return true if one of the states is true
///
/// If matching a transition, it'll check the first state matcher, and returns that
/// result unless it is `NoMatch`. In that case, it will return the result of the second
/// state matcher.
pub struct CombineStateMatchers<
    S: States,
    Sm1: StateMatcher<S, M1>,
    M1: 'static,
    Sm2: StateMatcher<S, M2>,
    M2: 'static,
>(
    pub Sm1,
    pub Sm2,
    PhantomData<Box<dyn Send + Sync + 'static + Fn(S) -> (M1, M2)>>,
);

impl<S: States, Sm1: StateMatcher<S, M1>, M1: 'static, Sm2: StateMatcher<S, M2>, M2: 'static>
    sealed::InternalStateMatcher<S, (M1, M2)> for CombineStateMatchers<S, Sm1, M1, Sm2, M2>
{
    fn match_state(&self, state: &S) -> bool {
        self.0.match_state(state) || self.1.match_state(state)
    }

    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let result = self.0.match_state_transition(main, secondary);
        if result != MatchesStateTransition::NoMatch {
            return result;
        }
        self.1.match_state_transition(main, secondary)
    }
}

pub(crate) mod sealed {
    use std::marker::PhantomData;

    use crate::schedule::States;

    use super::MatchesStateTransition;

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

    pub trait InternalStateMatcher<S: States, Marker>: Send + Sync + Sized + 'static {
        /// Check whether to match with the current state
        fn match_state(&self, state: &S) -> bool;

        /// Check whether to match a state transition
        fn match_state_transition(
            &self,
            main: Option<&S>,
            secondary: Option<&S>,
        ) -> MatchesStateTransition;
    }
}

use sealed::InternalStateMatcher;

/// A trait for matching `S: States` or transitions between two `S: States`.
///
/// Can only be used via the existing auto-implementations. Valid implementors include:
///
/// - `S` itself
/// - `Fn(&Self) -> bool`
/// - `Fn(Option<&Self>) -> bool`
/// - `Fn(&Self, &Self) -> bool`
/// - `Fn(&Self, Option<&Self>) -> bool`
/// - `Fn(Option<&Self>, Option<&Self>) -> bool`
/// - `Fn(&Self, &Self) -> MatchesStateTransition`
/// - `Fn(&Self, Option<&Self>) -> MatchesStateTransition`
/// - `Fn(Option<&Self>, Option<&Self>) -> MatchesStateTransition`
pub trait StateMatcher<S: States, Marker>: InternalStateMatcher<S, Marker> {
    /// Ensures that any transition is considered valid if the `main` state
    /// matches, regardless of anything else.
    fn every(self) -> EveryTransition<S, Self, Marker> {
        EveryTransition(self, PhantomData)
    }

    /// Swaps the `main` and `secondary` states when calling `match_state_transition`
    ///
    /// Can be used to focus on the incoming state when `Exiting`,
    /// or to focus on the outgoing state when `Entering`
    fn invert_transition(self) -> InvertTransition<S, Self, Marker> {
        InvertTransition(self, PhantomData)
    }

    /// Combines two `StateMatcher`s in order
    ///
    /// If matching a single state, it will return true if one of the states is true
    ///
    /// If matching a transition, it'll check the first state matcher, and returns that
    /// result unless it is `NoMatch`. In that case, it will return the result of the second
    /// state matcher.
    fn combine<M2, Sm: StateMatcher<S, M2>>(
        self,
        other: Sm,
    ) -> CombineStateMatchers<S, Self, Marker, Sm, M2> {
        CombineStateMatchers(self, other, PhantomData)
    }
}

impl<S: States, Marker, Sm: InternalStateMatcher<S, Marker>> StateMatcher<S, Marker> for Sm {}

/// Define a state matcher using a single state conditional
pub(crate) trait SingleStateMatcher<S: States, Marker: sealed::Marker>:
    Send + Sync + Sized + 'static
{
    /// Check whether to match with the current state
    fn match_single_state(&self, state: &S) -> bool;
}

/// Define a state matcher with custom transition logic
pub(crate) trait TransitionStateMatcher<S: States, Marker: sealed::Marker>:
    Send + Sync + Sized + 'static
{
    /// Check whether to match a state transition
    fn match_transition(&self, main: Option<&S>, secondary: Option<&S>) -> MatchesStateTransition;
}

impl<S: States, M: sealed::Marker, Matcher: SingleStateMatcher<S, M>>
    InternalStateMatcher<S, sealed::IsSingleStateMatcher<M>> for Matcher
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
    InternalStateMatcher<S, sealed::IsTransitionMatcher<M>> for Matcher
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
    InternalStateMatcher<S, sealed::IsFn<sealed::OptStateRef, sealed::BoolReturn>> for F
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
    InternalStateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::TransitionReturn>,
    > for F
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
    InternalStateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::TransitionReturn>,
    > for F
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
    InternalStateMatcher<S, sealed::IsFn<(sealed::StateRef, sealed::StateRef), sealed::BoolReturn>>
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
        self(main, secondary).into()
    }

    fn match_state(&self, _: &S) -> bool {
        false
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> bool>
    InternalStateMatcher<
        S,
        sealed::IsFn<(sealed::StateRef, sealed::OptStateRef), sealed::BoolReturn>,
    > for F
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
    InternalStateMatcher<
        S,
        sealed::IsFn<(sealed::OptStateRef, sealed::OptStateRef), sealed::BoolReturn>,
    > for F
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

impl<S: States, M: 'static, Sm: StateMatcher<S, M>> IntoSystem<(), bool, (S, M)> for Sm {
    type System = StateMatcherSystem<S, M, Sm>;

    fn into_system(this: Self) -> Self::System {
        let system = IntoSystem::into_system(
            move |main: Option<Res<State<S>>>, transition: Option<Res<ActiveTransition<S>>>| {
                if let Some(transition) = transition.as_ref().map(|v| v.as_ref()) {
                    let main = transition.get_main();
                    let secondary = transition.get_secondary();

                    if main == secondary {
                        false
                    } else {
                        let result = this.match_state_transition(main, secondary);
                        result == MatchesStateTransition::TransitionMatches
                    }
                } else if let Some(main) = main {
                    this.match_state(main.get())
                } else {
                    false
                }
            },
        );
        StateMatcherSystem(Box::new(system), PhantomData)
    }
}

/// A system type for `StateMatcher`s
/// Allows them to be used as `Condition`s directly
pub struct StateMatcherSystem<S: States, M: 'static, Sm: StateMatcher<S, M>>(
    Box<dyn crate::prelude::ReadOnlySystem<In = (), Out = bool>>,
    PhantomData<fn() -> (S, M, Sm)>,
);

impl<S: States, M: 'static, Sm: StateMatcher<S, M>> System for StateMatcherSystem<S, M, Sm> {
    type In = ();

    type Out = bool;

    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    fn type_id(&self) -> std::any::TypeId {
        self.0.type_id()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.0.component_access()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.0.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.0.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.0.is_exclusive()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out {
        self.0.run_unsafe(input, world)
    }

    fn apply_deferred(&mut self, world: &mut crate::prelude::World) {
        self.0.apply_deferred(world);
    }

    fn initialize(&mut self, world: &mut crate::prelude::World) {
        self.0.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.0.update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: crate::component::Tick) {
        self.0.check_change_tick(change_tick);
    }

    fn get_last_run(&self) -> crate::component::Tick {
        self.0.get_last_run()
    }

    fn set_last_run(&mut self, last_run: crate::component::Tick) {
        self.0.set_last_run(last_run);
    }
}

/// SAFETY: The boxed system is must be a read only system
unsafe impl<S: States, M: 'static, Sm: StateMatcher<S, M>> ReadOnlySystem
    for StateMatcherSystem<S, M, Sm>
{
}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::state_matches;

    use crate as bevy_ecs;
    use crate::schedule::ActiveTransition;
    use crate::system::{IntoSystem, System};
    use crate::{
        schedule::{MatchesStateTransition, State, States},
        world::World,
    };

    use super::sealed::InternalStateMatcher;

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

    fn only_c(state: &TestState) -> bool {
        matches!(state, TestState::C(_))
    }

    #[test]
    fn a_single_state_matcher_matches_all_relevant_variants() {
        assert!(only_c.match_state(&TestState::C(true)));
        assert!(only_c.match_state(&TestState::C(false)));
        assert!(!only_c.match_state(&TestState::A));
        assert!(!only_c.match_state(&TestState::B));
    }

    #[test]
    fn a_single_state_matcher_matches_transitions_in_and_out_of_a_match_and_not_within_it() {
        assert_eq!(
            only_c.match_state_transition(Some(&TestState::C(true)), Some(&TestState::A)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            only_c.match_state_transition(Some(&TestState::C(false)), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            only_c.match_state_transition(Some(&TestState::C(true)), Some(&TestState::C(false))),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            only_c.match_state_transition(Some(&TestState::A), Some(&TestState::C(false))),
            MatchesStateTransition::NoMatch
        );
    }

    fn a_to_b(main: Option<&TestState>, secondary: Option<&TestState>) -> MatchesStateTransition {
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

    #[test]
    fn a_transition_state_matcher_can_match_single_states() {
        assert!(a_to_b.match_state(&TestState::A));
        assert!(!a_to_b.match_state(&TestState::B));
    }

    #[test]
    fn a_transition_state_matcher_can_match_state_transitions() {
        assert_eq!(
            a_to_b.match_state_transition(Some(&TestState::A), Some(&TestState::B)),
            MatchesStateTransition::TransitionMatches
        );
        assert_eq!(
            a_to_b.match_state_transition(Some(&TestState::A), Some(&TestState::C(false))),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            a_to_b.match_state_transition(Some(&TestState::A), None),
            MatchesStateTransition::MainMatches
        );
        assert_eq!(
            a_to_b.match_state_transition(Some(&TestState::B), Some(&TestState::A)),
            MatchesStateTransition::NoMatch
        );
        assert_eq!(
            a_to_b.match_state_transition(None, Some(&TestState::B)),
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

        let match_state_value = state_matches!(TestState, =TestState::A);
        assert!(match_state_value.match_state(&state_a));
        assert!(!match_state_value.match_state(&state_b));
        let match_state_value = state_matches!(TestState, =only_c);
        assert!(match_state_value.match_state(&state_c));
        assert!(!match_state_value.match_state(&state_b));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_simple_pattern() {
        let state_a = State::new(TestState::A);
        let state_b = State::new(TestState::B);
        let state_c = State::new(TestState::C(true));
        let state_c2 = State::new(TestState::C(false));

        let match_state_value = state_matches!(TestState, C(_));
        assert!(match_state_value.match_state(&state_c));
        assert!(match_state_value.match_state(&state_c2));
        assert!(!match_state_value.match_state(&state_a));
        assert!(!match_state_value.match_state(&state_b));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_closure() {
        let state_a = State::new(TestState::A);
        let state_b = State::new(TestState::B);
        let state_c = State::new(TestState::C(true));
        let state_c2 = State::new(TestState::C(false));

        let match_state_value = state_matches!(TestState, |state: &TestState| matches!(
            state,
            TestState::C(_)
        ));
        assert!(match_state_value.match_state(&state_c));
        assert!(match_state_value.match_state(&state_c2));
        assert!(!match_state_value.match_state(&state_a));
        assert!(!match_state_value.match_state(&state_b));
    }

    #[test]
    fn macro_can_generate_matcher_for_a_simple_transition() {
        let mut world = World::new();

        let match_state_value = state_matches!(TestState, C(_));

        let mut system = IntoSystem::into_system(match_state_value);

        system.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::A),
        ));
        assert!(system.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::C(false)),
        ));
        assert!(!system.run((), &mut world));
    }
    #[test]
    fn macro_can_generate_matcher_for_every_transition() {
        let mut world = World::new();

        let match_state_value = state_matches!(TestState, every C(_));

        let mut system = IntoSystem::into_system(match_state_value);

        system.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::A),
        ));
        assert!(system.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::C(false)),
        ));
        assert!(system.run((), &mut world));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::A),
            Some(TestState::C(false)),
        ));
        world.insert_resource(State::new(TestState::A));
        assert!(!system.run((), &mut world));
    }
    #[test]
    fn macro_can_generate_multi_pattern_matcher() {
        let mut world = World::new();

        let match_state_value = state_matches!(TestState, C(_), every |_: &TestState| true);
        let match_state_value_alt = state_matches!(TestState, =only_c, every |_: &TestState| true);

        let mut system = IntoSystem::into_system(match_state_value);

        let mut system_alt = IntoSystem::into_system(match_state_value_alt);

        system.initialize(&mut world);
        system_alt.initialize(&mut world);
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::A),
        ));
        assert!(system.run((), &mut world));
        assert!(system_alt.run((), &mut world));
        world.insert_resource(State::new(TestState::C(true)));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::C(true)),
            Some(TestState::C(false)),
        ));
        assert!(!system.run((), &mut world));
        assert!(!system_alt.run((), &mut world));
        world.insert_resource(ActiveTransition::new(
            Some(TestState::A),
            Some(TestState::C(true)),
        ));
        world.insert_resource(State::new(TestState::A));
        assert!(system.run((), &mut world));
        assert!(system_alt.run((), &mut world));
    }
}
