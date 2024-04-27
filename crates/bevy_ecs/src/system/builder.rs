use std::marker::PhantomData;

use bevy_utils::{all_tuples, index_tuple};

use crate::{archetype::ArchetypeGeneration, prelude::World};

use super::{FunctionSystem, IntoSystem, SystemMeta, SystemParam, SystemParamFunction};

pub struct SystemBuilder<'w, Marker, F: SystemParamFunction<Marker>>
where
    Self: SystemParamBuilder<F::Param>,
{
    func: Option<F>,
    builders: <Self as SystemParamBuilder<F::Param>>::Builders,
    world: &'w mut World,
}

impl<'w, Marker: 'static, F: SystemParamFunction<Marker>> SystemBuilder<'w, Marker, F>
where
    Self: SystemParamBuilder<F::Param>,
{
    pub fn new(world: &'w mut World, func: F) -> Self {
        Self {
            func: Some(func),
            builders: <Self as SystemParamBuilder<F::Param>>::Builders::default(),
            world,
        }
    }

    pub fn param<const I: usize>(
        &mut self,
        build: impl FnMut(&mut <Self as SystemParamBuilderIndex<F::Param, Self, I>>::Builder<'_>)
            + 'static,
    ) -> &mut Self
    where
        Self: SystemParamBuilderIndex<F::Param, Self, I>,
    {
        <Self as SystemParamBuilderIndex<F::Param, Self, I>>::set_builder(
            &mut self.builders,
            build,
        );
        self
    }

    pub fn build(&mut self) -> FunctionSystem<Marker, F> {
        let mut system_meta = SystemMeta::new::<F>();
        let param_state = Some(<Self as SystemParamBuilder<F::Param>>::build(
            self.world,
            &mut system_meta,
            &mut self.builders,
        ));
        let system = std::mem::take(&mut self.func);
        FunctionSystem {
            func: system.expect("Tried to build system from a SystemBuilder twice."),
            param_state,
            system_meta,
            world_id: Some(self.world.id()),
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct IsBuiltSystem;

impl<'w, Marker: 'static, F: SystemParamFunction<Marker>>
    IntoSystem<F::In, F::Out, (IsBuiltSystem, Marker)> for SystemBuilder<'w, Marker, F>
where
    Self: SystemParamBuilder<F::Param>,
{
    type System = FunctionSystem<Marker, F>;
    fn into_system(mut builder: Self) -> Self::System {
        builder.build()
    }
}

#[doc(hidden)]
pub trait SystemParamBuilder<P: SystemParam> {
    type Builders: Default;

    fn build(
        world: &mut World,
        system_meta: &mut SystemMeta,
        builders: &mut Self::Builders,
    ) -> P::State;
}

#[doc(hidden)]
pub trait SystemParamBuilderIndex<P: SystemParam, B: SystemParamBuilder<P>, const I: usize> {
    type Param: SystemParam;
    type Builder<'b>;

    fn set_builder(builders: &mut B::Builders, build: impl FnMut(&mut Self::Builder<'_>) + 'static);
}

macro_rules! expr {
    ($x:expr) => {
        $x
    };
}

macro_rules! impl_system_param_builder_index {
    ($idx:tt, $param:ident, $($all:ident),*) => {
        impl<'w, Marker: 'static, $($all: SystemParam + 'static,)* F: SystemParamFunction<Marker, Param = ($($all,)*)>>
            SystemParamBuilderIndex<($($all,)*), SystemBuilder<'w, Marker, F>, { $idx }> for SystemBuilder<'w, Marker, F>
        {
            type Param = $param;
            type Builder<'b> = $param::Builder<'b>;

            fn set_builder(builders: &mut <SystemBuilder<'w, Marker, F> as SystemParamBuilder<($($all,)*)>>::Builders, build: impl FnMut(&mut Self::Builder<'_>) + 'static) {
                expr!(builders.$idx) = Some(Box::new(build));
            }
        }
    };
}

macro_rules! impl_system_param_builder {
    ($(($param: tt, $builder: ident)),*) => {
        impl<'w, Marker: 'static, $($param: SystemParam + 'static,)* F: SystemParamFunction<Marker, Param = ($($param,)*)>>
            SystemParamBuilder<($($param,)*)> for SystemBuilder<'w, Marker, F>
        {
            type Builders = ($(Option<Box<dyn FnMut(&mut $param::Builder<'_>)>>,)*);

            #[allow(non_snake_case)]
            fn build(_world: &mut World, _system_meta: &mut SystemMeta, _builders: &mut Self::Builders) -> <($($param,)*) as SystemParam>::State {
                let ($($builder,)*) = _builders;
                ($(
                    $builder.as_mut().map(|b| $param::build(_world, _system_meta, b))
                        .unwrap_or_else(|| $param::init_state(_world, _system_meta)),
                )*)
            }
        }

        index_tuple!(impl_system_param_builder_index, $($param),*);
    }
}

all_tuples!(impl_system_param_builder, 1, 12, P, B);

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::prelude::{Component, Query};
    use crate::system::{Local, RunSystemOnce};

    use super::*;

    #[derive(Component)]
    struct A;

    fn local_system(local: Local<u64>) -> u64 {
        *local
    }

    fn query_system(query: Query<()>) -> usize {
        query.iter().count()
    }

    #[test]
    fn local_builder() {
        let mut world = World::new();

        let system = SystemBuilder::new(&mut world, local_system)
            .param::<0>(|local| *local = 10)
            .build();

        let result = world.run_system_once(system);
        assert_eq!(result, 10);
    }

    #[test]
    fn query_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = SystemBuilder::new(&mut world, query_system)
            .param::<0>(|query| {
                query.with::<A>();
            })
            .build();

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }
}
