use smallvec::SmallVec;

use crate::{component::Component, prelude::Entity};

use super::{FetchState, Relation, WorldQuery};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

// NOTE: This whole file is ~~hilarious~~ elegant type system hacking- thanks to @TheRawMeatball for coming up with this :)

pub struct QueryTargetFilters<Q: WorldQuery, F: WorldQuery>(
    pub <Q::State as FetchState>::TargetFilter,
    pub <F::State as FetchState>::TargetFilter,
    PhantomData<fn() -> (Q, F)>,
);

macro_rules! impl_trait {
    ($trait:ident, $($body:tt)*) => {
        impl<Q: WorldQuery, F: WorldQuery> $trait for QueryTargetFilters<Q, F>
            where
                <Q::State as FetchState>::TargetFilter: $trait,
                <F::State as FetchState>::TargetFilter: $trait {
            $($body)*
        }
    };
}

impl_trait!(
    Clone,
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
);

impl_trait!(
    Default,
    fn default() -> Self {
        Self(Default::default(), Default::default(), PhantomData)
    }
);

impl_trait!(
    PartialEq,
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
);

impl_trait!(Eq,);

impl_trait!(
    Hash,
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
);

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct TargetFilter<K: Component>(SmallVec<[Entity; 4]>, PhantomData<K>);

impl<K: Component> TargetFilter<K> {
    pub fn new() -> Self {
        Self(SmallVec::new(), PhantomData)
    }

    pub fn target(mut self, target: Entity) -> Self {
        self.0.push(target);
        self
    }
}

impl<Q: WorldQuery, F: WorldQuery> QueryTargetFilters<Q, F> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_filter_relation<K: Component, Path>(&mut self, filter: TargetFilter<K>)
    where
        Self: SpecifiesRelation<K, Path, TargetFilter = Self>,
    {
        Self::__add_(filter, self);
    }

    pub fn deduplicate_targets(&mut self) {
        <Q::State as FetchState>::deduplicate_targets(&mut self.0);
        <F::State as FetchState>::deduplicate_targets(&mut self.1);
    }
}

pub trait SpecifiesRelation<Kind: Component, Path> {
    type TargetFilter;
    fn __add_(entity: TargetFilter<Kind>, target_filter: &mut Self::TargetFilter);
}

pub struct Intrinsic;
pub struct InData<Inner>(PhantomData<Inner>);
pub struct InFilter<Inner>(PhantomData<Inner>);
pub struct InTuple<Inner, const I: usize>(PhantomData<Inner>);

impl<Kind: Component> SpecifiesRelation<Kind, Intrinsic> for &Relation<Kind> {
    type TargetFilter = <<Self as WorldQuery>::State as FetchState>::TargetFilter;
    fn __add_(filter: TargetFilter<Kind>, target_filter: &mut smallvec::SmallVec<[Entity; 4]>) {
        target_filter.extend(filter.0.into_iter());
    }
}
impl<Kind: Component> SpecifiesRelation<Kind, Intrinsic> for &mut Relation<Kind> {
    type TargetFilter = <<Self as WorldQuery>::State as FetchState>::TargetFilter;
    fn __add_(filter: TargetFilter<Kind>, target_filter: &mut smallvec::SmallVec<[Entity; 4]>) {
        target_filter.extend(filter.0.into_iter());
    }
}
impl<Kind: Component> SpecifiesRelation<Kind, Intrinsic>
    for crate::prelude::Without<Relation<Kind>>
{
    type TargetFilter = <<Self as WorldQuery>::State as FetchState>::TargetFilter;
    fn __add_(filter: TargetFilter<Kind>, target_filter: &mut smallvec::SmallVec<[Entity; 4]>) {
        target_filter.extend(filter.0.into_iter());
    }
}
impl<Kind: Component> SpecifiesRelation<Kind, Intrinsic> for crate::prelude::With<Relation<Kind>> {
    type TargetFilter = <<Self as WorldQuery>::State as FetchState>::TargetFilter;
    fn __add_(filter: TargetFilter<Kind>, target_filter: &mut smallvec::SmallVec<[Entity; 4]>) {
        target_filter.extend(filter.0.into_iter());
    }
}

impl<Kind: Component, Path, Q: WorldQuery, F: WorldQuery> SpecifiesRelation<Kind, InData<Path>>
    for QueryTargetFilters<Q, F>
where
    Q: SpecifiesRelation<
        Kind,
        Path,
        TargetFilter = <<Q as WorldQuery>::State as FetchState>::TargetFilter,
    >,
{
    type TargetFilter = Self;
    fn __add_(entity: TargetFilter<Kind>, target_filter: &mut Self::TargetFilter) {
        Q::__add_(entity, &mut target_filter.0);
    }
}
impl<Kind: Component, Path, Q: WorldQuery, F: WorldQuery> SpecifiesRelation<Kind, InFilter<Path>>
    for QueryTargetFilters<Q, F>
where
    F: SpecifiesRelation<
        Kind,
        Path,
        TargetFilter = <<F as WorldQuery>::State as FetchState>::TargetFilter,
    >,
{
    type TargetFilter = Self;
    fn __add_(entity: TargetFilter<Kind>, target_filter: &mut Self::TargetFilter) {
        F::__add_(entity, &mut target_filter.1);
    }
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! count_tts {
    ($($tts:tt)*) => {0usize $(+ replace_expr!($tts 1usize))*};
}

macro_rules! impl_tuple_inner {
    ([$($head: ident),*], [$($tail: ident),*]) => {
        impl<Kind: Component, Inner, Selected, $($head,)* $($tail,)*>
            SpecifiesRelation<Kind, InTuple<Inner, { count_tts!($($head)*) }>>
            for
            ($($head,)* Selected, $($tail,)*)
        where
            $($head: WorldQuery,)*
            $($tail: WorldQuery,)*
            Selected: WorldQuery +
                SpecifiesRelation<
                    Kind,
                    Inner,
                    TargetFilter = <<Selected as WorldQuery>::State as FetchState>::TargetFilter,
                >,
        {
            type TargetFilter = (
                $(<<$head as WorldQuery>::State as FetchState>::TargetFilter,)*
                <Selected::State as FetchState>::TargetFilter,
                $(<<$tail as WorldQuery>::State as FetchState>::TargetFilter,)*
            );

            #[allow(non_snake_case, unused)]
            fn __add_(entity: TargetFilter<Kind>, target_filter: &mut Self::TargetFilter) {
                let (
                    $($head,)*
                    my_thing,
                    $($tail,)*
                ) = target_filter;
                Selected::__add_(entity, my_thing);
            }
        }
    };
}

macro_rules! impl_tuple {
    ($($idents: ident),*) => {
        impl_tuple!([], [$($idents),*]);
    };
    ([$($head: ident),*], []) => {
        impl_tuple_inner!([$($head),*], []);
    };
    ([$($head: ident),*], [$last: ident]) => {
        impl_tuple_inner!([$($head),*], [$last]);
        impl_tuple!([$($head,)* $last], []);
    };
    ([$($head: ident),*], [$transfer: ident, $($tail: ident),*]) => {
        impl_tuple_inner!([$($head),*], [$($tail,)* $transfer]);
        impl_tuple!([$($head,)* $transfer], [$($tail),*]);
    };
}

impl_tuple!();
impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);

#[cfg(test)]
#[test]
fn target_filter_tests() {
    fn assert_impl<Kind: Component, Path, T: SpecifiesRelation<Kind, Path> + ?Sized>() {}
    assert_impl::<u64, _, QueryTargetFilters<(&Relation<u32>, &Relation<u64>), ()>>();
    assert_impl::<u32, _, QueryTargetFilters<(&Relation<u32>, &Relation<u64>), ()>>();

    let mut filter: QueryTargetFilters<&Relation<u32>, ()> = Default::default();
    filter.add_filter_relation(TargetFilter::<u32>::new().target(Entity::new(1)));
    dbg!(&filter.0);

    let mut filter: QueryTargetFilters<(&Relation<u32>, &Relation<u64>), ()> = Default::default();
    filter.add_filter_relation(TargetFilter::<u32>::new().target(Entity::new(1)));
    filter.add_filter_relation(TargetFilter::<u64>::new().target(Entity::new(12)));
    dbg!(&filter.0);
}
