use std::marker::PhantomData;

use bevy_ecs::{
    entity::Entity,
    system::{SystemParam, SystemParamItem},
};

use super::{FeatureIO, FeatureSignature, Sig, SubFeature, SubFeatureInput, SubFeatureOutput};

//credit to SystemParamFunction impl

#[doc(hidden)]
pub struct IsFunctionSubFeature;

pub struct FunctionSubFeature<Marker, F: SubFeatureFunction<Marker>> {
    fun: F,
    data: PhantomData<fn() -> Marker>,
}

pub trait SubFeatureFunction<Marker>: Send + Sync + 'static {
    type Sig: FeatureSignature<false>;
    type Param: SystemParam;

    fn run(
        &self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature<false>>::In,
        param: &SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature<false>>::Out;
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
    fn run(
        &self,
        view_entity: Entity,
        input: SubFeatureInput<Self>,
        param: &SystemParamItem<Self::Param>,
    ) -> SubFeatureOutput<Self> {
        F::run(&self.fun, view_entity, input, param)
    }
}

impl<In, Out, Func: Send + Sync + 'static> SubFeatureFunction<fn(Entity, In) -> Out> for Func
where
    In: FeatureIO<true>,
    Out: FeatureIO<false>,
    for<'a> &'a Func: Fn(Entity, In) -> Out + Fn(Entity, In) -> Out,
{
    type Sig = Sig![In => Out];
    type Param = ();

    #[inline]
    fn run(
        &self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature<false>>::In,
        _param: &SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature<false>>::Out {
        // Yes, this is strange, but `rustc` fails to compile this impl
        // without using this function. It fails to recognize that `func`
        // is a function, potentially because of the multiple impls of `FnMut`
        fn call_inner<In, Out>(
            f: impl Fn(Entity, In) -> Out,
            view_entity: Entity,
            input: In,
        ) -> Out {
            f(view_entity, input)
        }
        call_inner(self, view_entity, input)
    }
}

impl<Out, In, Func: Send + Sync + 'static, Param: SystemParam>
    SubFeatureFunction<fn(Entity, In, Param) -> Out> for Func
where
    In: FeatureIO<true>,
    Out: FeatureIO<false>,
    for<'a> &'a Func:
        Fn(Entity, In, &Param) -> Out + Fn(Entity, In, &SystemParamItem<Param>) -> Out,
{
    type Sig = Sig![In => Out];
    type Param = Param;

    #[inline]
    fn run(
        &self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature<false>>::In,
        param: &SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature<false>>::Out {
        // Yes, this is strange, but `rustc` fails to compile this impl
        // without using this function. It fails to recognize that `func`
        // is a function, potentially because of the multiple impls of `FnMut`
        fn call_inner<In, Out, Param>(
            f: impl Fn(Entity, In, &Param) -> Out,
            view_entity: Entity,
            input: In,
            param: &Param,
        ) -> Out {
            f(view_entity, input, param)
        }
        call_inner(self, view_entity, input, param)
    }
}
