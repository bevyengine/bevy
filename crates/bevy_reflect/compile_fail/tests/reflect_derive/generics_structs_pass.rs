//@check-pass
use bevy_reflect::{GetField, Reflect};

#[derive(Reflect)]
#[reflect(from_reflect = false)]
struct Foo<T, U, S> {
    a: T,
    #[reflect(ignore)]
    _b: U,

    // check that duplicate types don't cause any compile errors
    _c: T,
    // check that when a type is both an active and inactive type, both trait bounds are used
    _d: U,

    #[reflect(ignore)]
    _e: S,
}

// check that we use the proper bounds when auto-deriving `FromReflect`
#[derive(Reflect)]
struct Bar<T, U: Default, S: Default> {
    a: T,
    #[reflect(ignore)]
    _b: U,
    _c: T,
    _d: U,
    #[reflect(ignore)]
    _e: S,
}

fn main() {
    let foo = Foo::<u32, usize, f32> {
        a: 1,
        _b: 2,
        _c: 3,
        _d: 4,
        _e: 5.0,
    };

    let _ = *foo.get_field::<u32>("a").unwrap();

    let bar = Bar::<u32, usize, f32> {
        a: 1,
        _b: 2,
        _c: 3,
        _d: 4,
        _e: 5.0,
    };

    let _ = *bar.get_field::<u32>("a").unwrap();
}
