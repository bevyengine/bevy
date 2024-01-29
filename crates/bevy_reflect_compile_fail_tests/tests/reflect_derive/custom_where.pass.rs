use bevy_reflect::{Reflect, FromType};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> FromType<T> for ReflectMyTrait {
    fn from_type() -> Self {
        Self
    }
}

#[derive(Reflect)]
#[reflect(MyTrait)]
#[reflect(where)]
pub struct Foo<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

#[derive(Reflect)]
#[reflect(where)]
#[reflect(MyTrait)]
pub struct Bar<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

fn main() {}
