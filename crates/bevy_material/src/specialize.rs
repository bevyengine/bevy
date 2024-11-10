use bevy_ecs::system::ReadOnlySystemParam;
use bevy_utils::HashMap;
use core::{hash::Hash, marker::PhantomData};

use crate::{material::Material, material_pipeline::MaterialPipeline};

pub trait Specialize {
    type Key: Clone + Hash + Eq + Send + Sync;
    type Item: Send + Sync;
}

pub type SpecializeMaterialContext<'a, M, P> =
    <<P as MaterialPipeline>::Specializer<M> as SpecializeMaterial>::Context<'a>;

pub struct Specializer<T: Specialize> {
    items: HashMap<T::Key, T::Item>,
}

impl<T: Specialize> Specializer<T> {}

pub struct SpecializedMaterial<M: Material<P>, P: MaterialPipeline>(PhantomData<fn(M, P)>);
