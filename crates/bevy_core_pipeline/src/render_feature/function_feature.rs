use std::marker::PhantomData;

use bevy_ecs::{
    entity::Entity,
    system::{SystemParam, SystemParamItem},
};

use super::{
    FeatureIO, FeatureSig, FeatureSignature, SubFeature, SubFeatureInput, SubFeatureOutput,
};

#[doc(hidden)]
pub struct IsFunctionSubFeature;

pub struct FunctionSubFeature<Marker, F: SubFeatureFunction<Marker>> {
    fun: F,
    data: PhantomData<fn() -> Marker>,
}

pub trait SubFeatureFunction<Marker>: Send + Sync + 'static {
    type Sig: FeatureSignature;
    type Param: SystemParam;

    fn run(
        &mut self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature>::In,
        param: &mut SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature>::Out;
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
        &mut self,
        view_entity: Entity,
        input: SubFeatureInput<Self>,
        param: &mut SystemParamItem<Self::Param>,
    ) -> SubFeatureOutput<Self> {
        F::run(&mut self.fun, view_entity, input, param)
    }
}

impl<In, Out, Func: Send + Sync + 'static> SubFeatureFunction<fn(Entity, In) -> Out> for Func
where
    In: FeatureIO,
    Out: FeatureIO,
    for<'a> &'a mut Func: FnMut(Entity, In) -> Out + FnMut(Entity, In) -> Out,
{
    type Sig = FeatureSig![In => Out];
    type Param = ();

    #[inline]
    fn run(
        &mut self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature>::In,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature>::Out {
        // Yes, this is strange, but `rustc` fails to compile this impl
        // without using this function. It fails to recognize that `func`
        // is a function, potentially because of the multiple impls of `FnMut`
        fn call_inner<In, Out>(
            mut f: impl FnMut(Entity, In) -> Out,
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
    In: FeatureIO,
    Out: FeatureIO,
    for<'a> &'a mut Func:
        FnMut(Entity, In, &Param) -> Out + FnMut(Entity, In, &SystemParamItem<Param>) -> Out,
{
    type Sig = FeatureSig![In => Out];
    type Param = Param;

    #[inline]
    fn run(
        &mut self,
        view_entity: Entity,
        input: <Self::Sig as FeatureSignature>::In,
        param: &mut SystemParamItem<Self::Param>,
    ) -> <Self::Sig as FeatureSignature>::Out {
        // Yes, this is strange, but `rustc` fails to compile this impl
        // without using this function. It fails to recognize that `func`
        // is a function, potentially because of the multiple impls of `FnMut`
        fn call_inner<In: FeatureIO, Out: FeatureIO, Param>(
            mut f: impl FnMut(Entity, In, &Param) -> Out,
            view_entity: Entity,
            input: In,
            param: &Param,
        ) -> Out {
            f(view_entity, input, param)
        }
        call_inner(self, view_entity, input, param)
    }
}
