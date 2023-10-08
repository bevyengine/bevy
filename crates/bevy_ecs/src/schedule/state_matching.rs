use std::ops::Deref;

use crate::{change_detection::Res, system::IntoSystem};

pub use bevy_ecs_macros::{entering, exiting, state_matches, transitioning, StateMatcher};

use super::{Condition, States};

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

/// Types that can match world-wide states.
pub trait StateMatcher<S: States, Marker = ()>: Send + Sync + Sized + 'static {
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
pub trait SingleStateMatcher<S: States, Marker = ()>: Send + Sync + Sized + 'static {
    /// Check whether to match with the current state
    fn match_single_state(&self, state: &S) -> bool;
}

/// Define a state matcher with custom transition logic
pub trait TransitionStateMatcher<S: States, Marker = ()>: Send + Sync + Sized + 'static {
    /// Check whether to match a state transition
    fn match_transition(&self, main: Option<&S>, secondary: Option<&S>) -> MatchesStateTransition;
}

impl<S: States, M, Matcher: SingleStateMatcher<S, M>> StateMatcher<S, (((), ()), (M, ()))>
    for Matcher
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

impl<S: States, M, Matcher: TransitionStateMatcher<S, M>> StateMatcher<S, ((), (M, ()), ())>
    for Matcher
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

impl<S: States> SingleStateMatcher<S, ((), (), ())> for S {
    fn match_single_state(&self, state: &S) -> bool {
        state == self
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S) -> bool> SingleStateMatcher<S, ((), ())> for F {
    fn match_single_state(&self, state: &S) -> bool {
        self(state)
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>) -> bool> StateMatcher<S, ((), ((), ()))>
    for F
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
    StateMatcher<S, ((), ())> for F
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
    StateMatcher<S, ((), (), ())> for F
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
    > TransitionStateMatcher<S, (((), ()), ())> for F
{
    fn match_transition(&self, main: Option<&S>, secondary: Option<&S>) -> MatchesStateTransition {
        self(main, secondary)
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, &S) -> bool>
    StateMatcher<S, (((), ()), ((), (), (), ()))> for F
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
    StateMatcher<S, ((), (), (), ())> for F
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
    StateMatcher<S, ((), (), (), ((), ()))> for F
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
