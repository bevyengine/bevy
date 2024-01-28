use bevy_reflect::{Reflect, FromType};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> FromType<T> for ReflectMyTrait {
    fn from_type() -> Self {
        Self
    }
}

// Reason: where clause cannot be used with #[reflect(MyTrait)]
#[derive(Reflect)]
#[reflect(MyTrait, where)]
pub struct Foo<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

// Reason: where clause cannot be used with #[reflect(MyTrait)]
#[derive(Reflect)]
#[reflect(where, MyTrait)]
pub struct Bar<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

fn main() {}