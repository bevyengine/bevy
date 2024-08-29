use bevy_reflect::{FromReflect, Reflect};

// Reason: Cannot have conflicting `from_reflect` attributes
#[derive(Reflect)]
#[reflect(from_reflect = false)]
#[reflect(from_reflect = true)]
//~^ ERROR: already set to false
struct Foo {
    value: String,
}

// Reason: Cannot have conflicting `from_reflect` attributes
#[derive(Reflect)]
#[reflect(from_reflect = true)]
#[reflect(from_reflect = false)]
//~^ ERROR: already set to true
struct Bar {
    value: String,
}

// Reason: Conflicting `FromReflect` implementations
#[derive(Reflect, FromReflect)]
//~^ ERROR: conflicting implementation
struct Baz {
    value: String,
}
