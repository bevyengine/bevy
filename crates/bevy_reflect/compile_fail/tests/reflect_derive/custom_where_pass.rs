//@check-pass
use bevy_reflect::{CreateTypeData, Reflect};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> CreateTypeData<T> for ReflectMyTrait {
    fn create_type_data(_: ()) -> Self {
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
