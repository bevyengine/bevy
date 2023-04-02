use crate::schedule::ScheduleLabel;

use crate as bevy_ecs;

use bevy_utils::all_tuples;
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
pub trait SubstateLabel
where
    Self: Sized,
{

    /// Create [SubstateLabelInFn] from the type of a function.
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
    fn with<F: VariadicFn<Ret, Args>, Ret, Args>(_f: &F) -> SubstateLabelInFn<Ret, Args, Self> {
        SubstateLabelInFn(PhantomData)
    }

    fn new<Ret, Args>() -> SubstateLabelInFn<Ret, Args, Self>  {
        SubstateLabelInFn(PhantomData)
    }

    fn new_fn<F: VariadicFn<Ret, Args>, Ret, Args>() -> SubstateLabelInFn<Ret, Args, Self> {
        SubstateLabelInFn(PhantomData)
    }

}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter;
impl SubstateLabel for OnEnter {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit;

impl SubstateLabel for OnExit {}
