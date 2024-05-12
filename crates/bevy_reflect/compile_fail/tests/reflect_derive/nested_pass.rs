//@check-pass
use bevy_reflect::Reflect;

mod nested_generics {
    use super::*;

    #[derive(Reflect)]
    struct Foo<T>(T);

    #[derive(Reflect)]
    struct Bar<T>(Foo<T>);

    #[derive(Reflect)]
    struct Baz<T>(Bar<Foo<T>>);
}
