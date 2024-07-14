//@check-pass
use bevy_reflect::{CreateTypeData, Reflect};

#[derive(Clone)]
struct ReflectMyTrait;

impl<T> CreateTypeData<T> for ReflectMyTrait {
    fn create_type_data(_: ()) -> Self {
        todo!()
    }
}

impl<T> CreateTypeData<T, i32> for ReflectMyTrait {
    fn create_type_data(_: i32) -> Self {
        todo!()
    }
}

impl<T> CreateTypeData<T, (i32, i32)> for ReflectMyTrait {
    fn create_type_data(_: (i32, i32)) -> Self {
        todo!()
    }
}

#[derive(Reflect)]
#[reflect(MyTrait)]
struct NoArgs;

#[derive(Reflect)]
#[reflect(MyTrait(1 + 2))]
struct OneArg;

#[derive(Reflect)]
#[reflect(MyTrait(1 + 2, 3 + 4))]
struct TwoArgs;
