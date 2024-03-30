mod function_feature;
use bevy_ecs::schedule::{
    InternedScheduleLabel, InternedSystemSet, IntoSystemConfigs, IntoSystemSet, SystemSet,
};
use bevy_render::settings::{WgpuFeatures, WgpuLimits};
pub use function_feature::*;

use bevy_ecs::component::{Component, ComponentDescriptor, ComponentId, StorageType};
use bevy_ecs::world::{EntityRef, EntityWorldMut, World};
use bevy_render::renderer::RenderContext;

use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Mutex;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{EntityCommand, SystemParam, SystemParamItem};
use bevy_render::render_graph::{Node, NodeRunError, RenderGraphContext, RenderSubGraph};
use bevy_utils::all_tuples;

pub trait Feature<G: RenderSubGraph>: Sized + Send + Sync + 'static {
    type Sig: RenderSignature;
    type CompatibilityKey;

    fn check_compatibility(
        &self,
        _features: WgpuFeatures,
        _limits: WgpuLimits,
    ) -> Compatibility<Self::CompatibilityKey> {
        Compatibility::Full
    }

    fn dependencies<'s, 'b: 's>(
        &'s self,
        _compatibility: Self::CompatibilityKey,
        mut _builder: FeatureDependencyBuilder<'b, G, Self>,
    ) -> RenderHandles<'b, FeatureInput<G, Self>> {
        FeatureInput::<G, Self>::default_render_handles::<'b>()
    }

    fn build<'s, 'b: 's>(
        &'s self,
        compatibility: Self::CompatibilityKey,
        builder: &'b mut FeatureBuilder<'b, G, Self>,
        inputs: RenderHandles<'b, FeatureInput<G, Self>>,
    ) -> RenderHandles<'b, FeatureOutput<G, Self>>;
}

pub enum Compatibility<T> {
    Full,
    Partial(T),
    None,
}

pub struct RenderHandle<'a, A: Send + Sync + 'static> {
    internal: RenderHandleInternal<A>,
    data: PhantomData<fn() -> &'a A>,
}

impl<'a, A: Send + Sync + 'static> Copy for RenderHandle<'a, A> {}
impl<'a, A: Send + Sync + 'static> Clone for RenderHandle<'a, A> {
    fn clone(&self) -> Self {
        *self
    }
}

enum RenderHandleInternal<A: Send + Sync + 'static> {
    Hole,
    From {
        source: InternedSystemSet,
        component_id: RenderComponentId<A>,
    },
}

impl<A: Send + Sync + 'static> Copy for RenderHandleInternal<A> {}
impl<A: Send + Sync + 'static> Clone for RenderHandleInternal<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, A: Send + Sync + 'static> RenderHandle<'a, A> {
    pub fn hole() -> Self {
        Self {
            internal: RenderHandleInternal::Hole,
            data: PhantomData,
        }
    }

    fn new<Marker>(source: impl IntoSystemSet<Marker>, component_id: RenderComponentId<A>) -> Self {
        Self {
            internal: RenderHandleInternal::From {
                source: source.into_system_set().intern(),
                component_id,
            },
            data: PhantomData,
        }
    }
}

pub struct FeatureBuilder<'w, G: RenderSubGraph, F: Feature<G>> {
    app: &'w mut App,
    data: PhantomData<fn(G, F)>,
}

impl<'w, G: RenderSubGraph, F: Feature<G>> FeatureBuilder<'w, G, F> {
    pub fn add_sub_feature<'a, M, S: IntoSubFeature<M>>(
        &'a mut self,
        input: RenderHandles<'a, SubFeatureInput<S::SubFeature>>,
        sub_feature: S,
    ) -> RenderHandles<'a, SubFeatureOutput<S::SubFeature>> {
        todo!()
    }

    pub fn app(&'w mut self) -> &'w mut App {
        self.app
    }

    // pub fn add_systems<M>(
    //     &mut self,
    //     schedule: InternedScheduleLabel,
    //     systems: impl IntoSystemConfigs<M>,
    // ) -> &mut Self {
    //     self
    // }

    // pub fn map<'a, A: FeatureIO<false>, B: FeatureIO<false>>(
    //     &'a mut self,
    //     handles: RenderHandles<'a, false, A>,
    //     f: impl for<'_w> FnMut(A::Item<'_w>) -> B,
    // ) -> RenderHandle<'a, B> {
    //     todo!()
    // }
    //
    // pub fn map_many<'a, A: FeatureIO<true>, B: FeatureIO<false>>(
    //     &'a mut self,
    //     handles: RenderHandles<'a, true, A>,
    //     f: impl for<'_w> FnMut(A::Item<'_w>) -> B,
    // ) -> RenderHandle<'a, B> {
    //     self.app
    //         .world
    //         .init_component_with_descriptor(ComponentDescriptor::new::<FeatureComponent<B>>());
    //     //register system to map from input to output;
    //     todo!()
    // }
}

pub type RenderHandles<'a, A> = <A as RenderIO>::Handles<'a>;
type ComponentIds<A> = <A as RenderIO>::ComponentIds;
pub type RenderIOItem<'w, T> = <T as RenderIO>::Item<'w>;

pub struct RenderDependencyError {
    holes: Vec<(u8, TypeId)>,
}

pub trait RenderIO: Sized + Send + Sync + 'static {
    type ComponentIds: Send + Sync + 'static;
    type Handles<'a>: Send + Sync + 'a;
    type Item<'w>: Send + Sync + 'w;

    //named as such to prevent collisions
    fn feature_io_get_from_entity(
        entity: EntityRef<'_>,
        ids: Self::ComponentIds,
    ) -> Option<<Self as RenderIO>::Item<'_>>;

    fn default_render_handles<'a>() -> Self::Handles<'a>;

    fn ids_from_handles(
        handles: Self::Handles<'_>,
    ) -> Result<Self::ComponentIds, RenderDependencyError>;
}

macro_rules! impl_feature_io {
    ($(($T: ident, $r: ident, $h: ident)),*) => {
        impl <$($T: Send + Sync + 'static),*> RenderIO for ($($T,)*) {
            type ComponentIds = ($(RenderComponentId<$T>,)*);
            type Handles<'a> = ($(RenderHandle<'a, $T>,)*);
            type Item<'w> = ($(&'w $T,)*);

            #[allow(unused_variables, unreachable_patterns)]
            fn feature_io_get_from_entity(
                entity: EntityRef<'_>,
                ($($h,)*): Self::ComponentIds,
            ) -> Option<RenderIOItem<'_, Self>> {
                match ($($h.get_from_ref(entity),)*) {
                    ($(Some($r),)*) => Some(($($r,)*)),
                    _ => None,
                }
            }

            #[allow(clippy::unused_unit)]
            fn default_render_handles<'a>() -> Self::Handles<'a> {
                ($(RenderHandle::<$T>::hole(),)*)
            }

            #[allow(unused_variables, unreachable_patterns, unused_mut, unused_assignments)]
            fn ids_from_handles(
                handles: Self::Handles<'_>,
            ) -> Result<Self::ComponentIds, RenderDependencyError> {
                let ($($h,)*) = handles;
                match ($($h.internal,)*) {
                    ($(RenderHandleInternal::From{ component_id: $r, .. },)*) => Ok((($($r,)*))),
                    _ => {
                        let mut holes = Vec::new();
                        let mut i: u8 = 0;
                        $(if let RenderHandleInternal::Hole = $h.internal {
                            holes.push((i, TypeId::of::<$T>()));
                        }
                        i += 1;
                        )*
                        Err(RenderDependencyError { holes })
                    },
                }
            }
        }
    };
}

all_tuples!(impl_feature_io, 0, 16, T, r, h);

pub trait RenderSignature: 'static {
    type In: RenderIO;
    type Out: RenderIO;
}

impl<I: RenderIO, O: RenderIO> RenderSignature for (I, O) {
    type In = I;
    type Out = O;
}

#[macro_export]
macro_rules! RenderSig_Macro {
    [$i: ty => $o: ty] => {
        ($i, $o)
    };
}

pub use RenderSig_Macro as Sig;

type FeatureInput<G, F> = <<F as Feature<G>>::Sig as RenderSignature>::In;
type FeatureOutput<G, F> = <<F as Feature<G>>::Sig as RenderSignature>::Out;
type SubFeatureInput<F> = <<F as SubFeature>::Sig as RenderSignature>::In;
type SubFeatureOutput<F> = <<F as SubFeature>::Sig as RenderSignature>::Out;

pub trait SubFeature: Send + Sync + 'static {
    type Sig: RenderSignature;
    type Param: SystemParam;

    fn run<'w, 's>(
        &'s mut self,
        view_entity: Entity,
        input: RenderIOItem<'w, SubFeatureInput<Self>>,
        param: SystemParamItem<'w, 's, Self::Param>,
    ) -> SubFeatureOutput<Self>;
}

pub trait IntoSubFeature<Marker> {
    type SubFeature: SubFeature;

    fn into_sub_feature(self) -> Self::SubFeature;
}

impl<T: SubFeature> IntoSubFeature<()> for T {
    type SubFeature = Self;
    #[inline]
    fn into_sub_feature(self) -> Self {
        self
    }
}

//implementation time!!!!!

struct SubFeatureNode<G: RenderSubGraph, F: Feature<G>, S: SubFeature> {
    sub_feature: Mutex<S>, //todo: better storage?

    data: PhantomData<(G, F)>,
}

impl<G: RenderSubGraph, F: Feature<G>, S: SubFeature> Node for SubFeatureNode<G, F, S> {
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

pub struct FeatureDependencyBuilder<'w, G: RenderSubGraph, F: Feature<G>> {
    app: &'w mut App,
    data: PhantomData<fn(G, F)>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderComponentId<T: Send + Sync + 'static> {
    id: ComponentId,
    data: PhantomData<fn() -> T>,
}

impl<A: Send + Sync + 'static> Copy for RenderComponentId<A> {}
impl<A: Send + Sync + 'static> Clone for RenderComponentId<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> RenderComponentId<T> {
    pub fn new(world: &mut World, component: T) -> Self {
        // let id =
        //     world.init_component_with_descriptor(ComponentDescriptor::new::<FeatureComponent<T>>());
        // Self {
        //     id,
        //     data: PhantomData,
        // }
        todo!()
    }

    pub fn get_from_ref<'a>(&self, entity_ref: EntityRef<'a>) -> Option<&'a T> {
        // entity_ref
        //     .get_by_id(self.id)
        //     //SAFETY: by construction the internal id should match the layout of the component type
        //     .map(|ptr| unsafe { ptr.deref::<T>() })
        todo!()
    }

    pub fn insert_to_entity(&self, entity_mut: &mut EntityWorldMut<'_>, component: T) {
        //SAFETY: by construction the internal id should match the layout of the component type
        // OwningPtr::make(component, |ptr| unsafe {
        //     entity_mut.insert_by_id(self.id, ptr)
        // });
        todo!()
    }
}

struct InsertRenderComponent<T: Send + Sync + 'static> {
    pub component_id: RenderComponentId<T>,
    pub component: T,
}

impl<T: RenderIO> EntityCommand for InsertRenderComponent<T> {
    fn apply(self, id: Entity, world: &mut World) {
        let mut entity_mut = world.entity_mut(id);
        self.component_id
            .insert_to_entity(&mut entity_mut, self.component);
    }
}

// struct SubFeatureSystem<S: SubFeature> {
//     input: RawRenderHandles<true, SubFeatureInput<S>>,
//     output: RawRenderHandle<SubFeatureOutput<S>>,
//     sub_feature: S,
//     query_state: QueryState<FilteredEntityRef<'static>, With<ExtractedView>>,
//     system_state: SystemState<S::Param>,
// }
//
// impl<S: SubFeature> System for SubFeatureSystem<S> {
//     type In = ();
//
//     type Out = ();
//
//     fn name(&self) -> Cow<'static, str> {
//         let name_str = self.system_state.meta().name().to_owned(); //bad clone bc of the stuff
//         name_str.into()
//     }
//
//     fn component_access(&self) -> &Access<ComponentId> {
//         todo!()
//     }
//
//     fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
//         todo!()
//     }
//
//     fn is_send(&self) -> bool {
//         todo!()
//     }
//
//     fn is_exclusive(&self) -> bool {
//         todo!()
//     }
//
//     fn has_deferred(&self) -> bool {
//         todo!()
//     }
//
//     unsafe fn run_unsafe(
//         &mut self,
//         input: Self::In,
//         world: unsafe_world_cell::UnsafeWorldCell,
//     ) -> Self::Out {
//         todo!()
//     }
//
//     fn apply_deferred(&mut self, world: &mut World) {
//         todo!()
//     }
//
//     fn initialize(&mut self, _world: &mut World) {
//         todo!()
//     }
//
//     fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
//         todo!()
//     }
//
//     fn check_change_tick(&mut self, change_tick: Tick) {
//         todo!()
//     }
//
//     fn get_last_run(&self) -> Tick {
//         todo!()
//     }
//
//     fn set_last_run(&mut self, last_run: Tick) {
//         todo!()
//     }
// }

impl<'w, G: RenderSubGraph, F: Feature<G>> FeatureDependencyBuilder<'w, G, F> {
    // pub fn with_dep<'a: 'w, O: Feature<G>>(&'a mut self) -> RenderHandles<'a, FeatureOutput<G, O>> {
    //     todo!()
    // }
    //
    // pub fn map<'a, A: RenderIO, B: RenderIO>(
    //     &'a mut self,
    //     handles: RenderHandles<'a, A>,
    //     f: impl for<'_w> FnMut(A::Item<'_w>) -> B,
    // ) -> RenderHandle<'a, B> {
    //     todo!()
    // }
    //
    // pub fn map_many<'a, A: RenderIO, B: RenderIO>(
    //     &'a mut self,
    //     handles: RenderHandles<'a, A>,
    //     f: impl for<'_w> FnMut(A::Item<'_w>) -> B,
    // ) -> RenderHandle<'a, B> {
    //     // self.app
    //     //     .world
    //     //     .init_component_with_descriptor(ComponentDescriptor::new::<FeatureComponent<B>>());
    //     //register system to map from input to output;
    //     todo!()
    // }
}

#[macro_export]
macro_rules! Handles_Impl {
    ($($h: tt),*) => {
        ($($crate::render_feature::SingleHandle!($h)),*)
    }
}

pub use Handles_Impl as Handles;

#[macro_export]
macro_rules! SingleHandle_Impl {
    (_) => {
        $crate::render_feature::RenderHandle::hole()
    };
    ($h: expr) => {
        $h
    };
}

pub use SingleHandle_Impl as SingleHandle;
