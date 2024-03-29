use std::marker::PhantomData;

use bevy_ecs::{
    entity::Entity,
    system::{SystemParam, SystemParamItem},
};
use bevy_utils::all_tuples;

use super::{
    RenderIO, RenderIOItem, RenderSignature, Sig, SubFeature, SubFeatureInput, SubFeatureOutput,
};

//credit to SystemParamFunction impl

#[doc(hidden)]
pub struct IsFunctionSubFeature;

pub struct FunctionSubFeature<Marker, F: SubFeatureFunction<Marker>> {
    fun: F,
    data: PhantomData<fn() -> Marker>,
}

pub trait SubFeatureFunction<Marker>: Send + Sync + 'static {
    type Sig: RenderSignature<false>;
    type Param: SystemParam;

    fn run(
        &mut self,
        view_entity: Entity,
        input: RenderIOItem<'_, true, <Self::Sig as RenderSignature<false>>::In>,
        param: SystemParamItem<Self::Param>,
    ) -> <Self::Sig as RenderSignature<false>>::Out;
}

impl<Marker: 'static, F: SubFeatureFunction<Marker>>
    super::IntoSubFeature<(IsFunctionSubFeature, Marker)> for F
{
    type SubFeature = FunctionSubFeature<Marker, F>;
    #[inline]
    fn into_sub_feature(self) -> Self::SubFeature {
        FunctionSubFeature {
            fun: self,
            data: PhantomData,
        }
    }
}

impl<Marker: 'static, F: SubFeatureFunction<Marker>> SubFeature for FunctionSubFeature<Marker, F> {
    type Sig = F::Sig;

    type Param = F::Param;

    #[inline]
    fn run<'w, 's>(
        &'s mut self,
        view_entity: Entity,
        input: RenderIOItem<'w, true, SubFeatureInput<Self>>,
        param: SystemParamItem<'w, 's, Self::Param>,
    ) -> SubFeatureOutput<Self> {
        F::run(&mut self.fun, view_entity, input, param)
    }
}

macro_rules! impl_sub_feature_function {
    ($(($P: ident, $p: ident)),*) => {
        impl<Out, In, Func: Send + Sync + 'static, $($P: SystemParam),*>
            SubFeatureFunction<fn(Entity, In, $($P),*) -> Out> for Func
        where
            In: RenderIO<true>,
            Out: RenderIO<false>,
            for<'a, 'w> &'a mut Func: FnMut(Entity, In::Item<'w>, $($P),*) -> Out
                + FnMut(Entity, In::Item<'w>, $(SystemParamItem<$P>),*) -> Out,
        {
            type Sig = Sig![In => Out];
            type Param = ($($P,)*);

            #[inline]
            fn run(
                &mut self,
                view_entity: Entity,
                input: In::Item<'_>,
                ($($p,)*): SystemParamItem<Self::Param>,
            ) -> <Self::Sig as RenderSignature<false>>::Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<In: RenderIO<true>, Out, $($P),*>(
                    mut f: impl for<'w> FnMut(Entity, In::Item<'w>, $($P),*) -> Out,
                    view_entity: Entity,
                    input: In::Item<'_>,
                    $($p: $P),*
                ) -> Out {
                    f(view_entity, input, $($p),*)
                }
                call_inner(self, view_entity, input, $($p),*)
            }
        }
    };
}

all_tuples!(impl_sub_feature_function, 0, 16, T, p);
