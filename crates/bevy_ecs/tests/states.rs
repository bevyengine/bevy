
use bevy_ecs_macros::States;

#[test]
fn fieldful_and_fieldless_states() {
    #[derive(Hash, Eq, PartialEq, Clone, Debug, States)]
    pub enum Foo {
        Fieldless,
        Fieldful(Bar),
    }
    #[derive(Hash, Eq, PartialEq, Clone, Debug, States)]
    pub enum Bar {
        Alice,
        Bob,
    }
    impl Default for Bar {
        fn default() -> Self {
            Self::Alice
        }
    }
    impl Default for Foo {
        fn default() -> Self {
            Self::Fieldless
        }
    }

    assert_eq!(
        Foo::variants().collect::<Vec<Foo>>(),
        vec![
            Foo::Fieldless,
            Foo::Fieldful(Bar::Alice),
            Foo::Fieldful(Bar::Bob)
        ]
    )
}

#[test]
fn fieldless_inner() {
    #[derive(Hash, Eq, PartialEq, Clone, Debug)]
    pub enum Foo {
        Fieldless,
        Fieldful(Bar),
    }
    #[derive(Hash, Eq, PartialEq, Clone, Debug, States)]
    pub enum Bar {
        Alice,
        Bob,
    }
    impl Default for Bar {
        fn default() -> Self {
            Self::Alice
        }
    }
    let mut fields = vec![Foo::Fieldless];
    let fieldful = [Bar::variants().map(|variant| Foo::Fieldful(variant))];
    for field in fieldful {
        fields.extend(field)
    }

    fields.into_iter()
}
