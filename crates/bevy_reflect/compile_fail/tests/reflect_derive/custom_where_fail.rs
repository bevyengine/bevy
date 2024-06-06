use bevy_reflect::{CreateTypeData, Reflect};
use std::marker::PhantomData;

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> CreateTypeData<T> for ReflectMyTrait {
    fn create_type_data(_: ()) -> Self {
        Self
    }
}

// Reason: populated `where` clause must be last with #[reflect(MyTrait)]
#[derive(Reflect)]
#[reflect(where T: std::fmt::Debug, MyTrait)]
//~^ ERROR: /expected.+:/
// TODO: Investigate a way to improve the error message.
pub struct Foo<T> {
    value: String,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}
