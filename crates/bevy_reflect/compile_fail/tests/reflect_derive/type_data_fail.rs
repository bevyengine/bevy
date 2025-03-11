//@no-rustfix
use bevy_reflect::{CreateTypeData, Reflect};

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> CreateTypeData<T, f32> for ReflectMyTrait {
    fn create_type_data(_: f32) -> Self {
        todo!()
    }
}

#[derive(Reflect)]
#[reflect(MyTrait)]
//~^ ERROR: mismatched types
struct RequiredArgs;

#[derive(Reflect)]
#[reflect(MyTrait(123))]
//~^ ERROR: mismatched types
struct WrongArgs;
