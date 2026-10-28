use bevy_reflect::{FromDynamic, Reflect};

// Reason: Cannot have conflicting `from_dynamic` attributes
#[derive(Reflect)]
#[reflect(from_dynamic = false)]
#[reflect(from_dynamic = true)]
//~^ ERROR: already set to false
struct Foo {
    value: String,
}

// Reason: Cannot have conflicting `from_dynamic` attributes
#[derive(Reflect)]
#[reflect(from_dynamic = true)]
#[reflect(from_dynamic = false)]
//~^ ERROR: already set to true
struct Bar {
    value: String,
}

// Reason: Conflicting `FromDynamic` implementations
#[derive(Reflect, FromDynamic)]
//~^ ERROR: conflicting implementation
struct Baz {
    value: String,
}
