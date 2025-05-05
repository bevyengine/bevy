use alloc::boxed::Box;
use bevy_ecs::component::Component;
use bevy_platform::hash::FixedHasher;
use core::{
    any::Any,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

use crate::component::{ComponentId, ComponentsRegistrator, StorageType};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SharedComponentStorage;

pub struct SharedComponents {}

pub trait SharedComponentKey: Any {
    fn eq_key(&self, other: &dyn SharedComponentKey) -> bool;
    fn hash_key(&self) -> u64;
    fn clone_key(&self) -> Box<dyn SharedComponentKey>;
}

impl<T> SharedComponentKey for T
where
    T: Component + Eq + Hash + Any + Clone,
{
    fn eq_key(&self, other: &dyn SharedComponentKey) -> bool {
        let other: &dyn Any = other;
        if let Some(other) = other.downcast_ref::<T>() {
            return self == other;
        }
        false
    }

    fn hash_key(&self) -> u64 {
        FixedHasher.hash_one(&self)
    }

    fn clone_key(&self) -> Box<dyn SharedComponentKey> {
        Box::new(self.clone())
    }
}

impl SharedComponentKey for () {
    fn eq_key(&self, other: &dyn SharedComponentKey) -> bool {
        (other as &dyn Any).is::<Self>()
    }

    fn hash_key(&self) -> u64 {
        FixedHasher.hash_one(&self)
    }

    fn clone_key(&self) -> Box<dyn SharedComponentKey> {
        Box::new(*self)
    }
}

impl PartialEq for Box<dyn SharedComponentKey> {
    fn eq(&self, other: &Self) -> bool {
        SharedComponentKey::eq_key(self.as_ref(), other.as_ref())
    }
}

impl Eq for Box<dyn SharedComponentKey> {}

impl Hash for Box<dyn SharedComponentKey> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let key_hash = SharedComponentKey::hash_key(self.as_ref());
        state.write_u64(key_hash);
    }
}

pub trait BundleRegisterByValue {}

impl<T> BundleRegisterByValue for T where T: Eq + Hash + Any + Clone + Component {}

pub trait SuperKeyTrait {
    type RegisterComponent: Component;
    type KeyType: SharedComponentKey;
    type ValueType: Component;
    fn make_value_from_key(key: &Self::KeyType) -> Self::ValueType;
}

pub struct SelfKey<C: Component + SharedComponentKey>(PhantomData<C>);
impl<C: Component + SharedComponentKey + Clone> SuperKeyTrait for SelfKey<C> {
    type RegisterComponent = C;
    type KeyType = C;
    type ValueType = C;
    fn make_value_from_key(key: &Self::KeyType) -> Self::ValueType {
        Clone::clone(key)
    }
}

pub struct NoKey<C: Component>(PhantomData<C>);
impl<C: Component> SuperKeyTrait for NoKey<C> {
    type RegisterComponent = C;
    type KeyType = ();
    type ValueType = C;
    fn make_value_from_key(_key: &Self::KeyType) -> Self::ValueType {
        panic!("NoKey: No key value")
    }
}

pub struct ComponentKey<C: Component, K: SharedComponentKey + Component>(PhantomData<(C, K)>);
impl<C: Component, K: SharedComponentKey + Component> SuperKeyTrait for ComponentKey<C, K> {
    type RegisterComponent = C;
    type KeyType = K;
    type ValueType = C;
    fn make_value_from_key(_key: &Self::KeyType) -> Self::ValueType {
        unimplemented!()
    }
}

pub struct KeyFor<C, V>(PhantomData<(C, V)>);
impl<C: SharedComponentKey + Component, V: Component> SuperKeyTrait for KeyFor<C, V> {
    type RegisterComponent = V;
    type KeyType = ();
    type ValueType = C;
    fn make_value_from_key(_key: &Self::KeyType) -> Self::ValueType {
        panic!("KeyFor: No key value")
    }
}
