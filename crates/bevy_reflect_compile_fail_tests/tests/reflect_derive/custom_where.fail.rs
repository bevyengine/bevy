use bevy_reflect::{FromType, Reflect};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> FromType<T> for ReflectMyTrait {
    fn from_type() -> Self {
        Self
    }
}

// Reason: populated `where` clause must be last with #[reflect(MyTrait)]
#[derive(Reflect)]
#[reflect(where T: std::fmt::Debug, MyTrait)]
pub struct Foo<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

fn main() {}
