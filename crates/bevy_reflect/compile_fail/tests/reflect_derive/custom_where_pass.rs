//@check-pass
use bevy_reflect::{FromType, Reflect};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> FromType<T> for ReflectMyTrait {
    fn from_type() -> Self {
        Self
    }
}

#[derive(Reflect)]
#[reflect(MyTrait, where T: std::fmt::Debug)]
pub struct Foo<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

#[derive(Reflect)]
#[reflect(where, MyTrait)]
pub struct Bar<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

#[derive(Reflect)]
#[reflect(MyTrait)]
#[reflect(where T: std::fmt::Debug)]
pub struct Baz<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}
