use crate::schedule::ScheduleLabel;

use crate as bevy_ecs;
pub use bevy_ecs_macros::SubstateLabel;
use bevy_utils::{all_tuples, define_boxed_label};
use std::marker::PhantomData;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
///
///
/// This should be able to be used on some unnamed fields in enums. However, please beware that fields that have the same substate might pass the type checking, as they really match type-wise. (
/// So
/// ```rs
/// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
/// pub enum MyState {
///     Foo(AliceOrBob)
///     Bar(AliceOrBob)
/// }
///
/// pub AliceOrBob {
///     Alice,
///     Bob
/// }
/// // these should compile
/// let mut foo: Fn(AliceOrBob) -> MyState = MyState::Foo;
/// foo = MyState::Bar;
///
/// ```
pub struct SubstateLabelInFn<Ret, Args, L: SubstateLabel>(PhantomData<(L, Ret, Args)>);

pub struct SubstateInFn<L: SubstateLabel>(PhantomData<L>);
impl<L: SubstateLabel> SubstateInFn<L> {
    /// from the type of F from the value passed into the function
    pub fn new<F: VariadicFn<Ret, Args>, Ret, Args>(_f: &F) -> SubstateLabelInFn<Ret, Args, L> {
        SubstateLabelInFn(PhantomData)
    }
}

/// A trait which most variadic functions should technically implement?
/// The issue with Fn trait types is that it doesn't support variadic functions.  This one should
pub trait VariadicFn<Ret, Args> {}

macro_rules! impl_substate_tuple {
    ($($name: tt),*)  => {
        impl<Function, Ret, $( $name, )* >  VariadicFn<Ret, ($($name,)*)> for Function where Function: Fn($($name,)*) -> Ret {}
    }
}

pub trait Arguments {}
macro_rules! impl_args_tuple {
    ($($name: tt),*) => {
        impl< $( $name, )* > Arguments for  ($( $name, )* ) {

        }
    }
}

all_tuples!(impl_args_tuple, 0, 16, A);
all_tuples!(impl_substate_tuple, 0, 16, S);
pub trait SubstateLabel {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit;
