//! This is a temporary stress test of various bsn! features.
// TODO: move these into actual tests and replace this with a more instructive user-facing example
use bevy::{
    prelude::*,
    scene2::prelude::{Scene, *},
};
use bevy_scene2::SceneList;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (print::<Sprite>, print::<Transform>, print::<Team>))
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    assets.load_scene("scene://base.bsn", base());
    assets.load_scene("scene://transform_1000.bsn", transform_1000());
    let top_level_handle = assets.load_scene("scene://top_level.bsn", top_level());
    commands.spawn(ScenePatchInstance(top_level_handle));
}

fn top_level() -> impl Scene {
    let a = 20usize;
    let b = 1993usize;
    bsn! {
        #TopLevel
        :"scene://base.bsn"
        :x
        Sprite { size: b }
        Team::Green(10)
        {transform_1337()}
        Children [
            Sprite { size: {4 + a}}
        ]
    }
}

fn base() -> impl Scene {
    let sprites = (0..10usize)
        .map(|i| bsn! {Sprite { size: {i} }})
        .collect::<Vec<_>>();

    bsn! {
        Name("Base")
        Sprite {
            handle: "asset://branding/bevy_bird_dark.png",
            size: 1,
            nested: Nested {
                handle: @"asset://hello.png"
            }
        }
        Transform::from_translation(Vec3::new(1.0, 1.0, 1.0))
        Team::Red {
            x: 10,
            y: Nested {
                foo: 10
            },
        }
        Gen::<usize> {
            value: 10,
        }
        on(|event: On<Explode>| {
        })
        Foo(100, @"asset://branding/bevy_bird_dark.png")
        [
            (:sprite_big Sprite { size: 2 }),
            :widget(bsn_list![Text::new("hi")]),
            {sprites},
        ]
    }
}

fn sprite_big() -> impl Scene {
    bsn! {
        Sprite { size: 100000, handle: "asset://branding/icon.png" }
    }
}

fn x() -> impl Scene {
    bsn! {
        :"scene://transform_1000.bsn"
        Transform { translation: Vec3 { x: 11.0 } }
    }
}

fn transform_1000() -> impl Scene {
    bsn! {
        Transform {
            translation: Vec3 { x: 1000.0, y: 1000.0 }
        }
    }
}

fn transform_1337() -> impl Scene {
    bsn! {
        Transform {
            translation: Vec3 { x: 1337.0 }
        }
    }
}

#[derive(Component, Debug, GetTemplate)]
struct Sprite {
    handle: Handle<Image>,
    size: usize,
    entity: Entity,
    nested: Nested,
}

#[derive(Component, Debug, GetTemplate)]
struct Gen<T: GetTemplate<Template: Default + Template<Output = T>>> {
    size: usize,
    value: T,
}

#[derive(Clone, Debug, GetTemplate)]
struct Nested {
    foo: usize,
    #[template]
    handle: Handle<Image>,
}

#[derive(Component, Clone, Debug, GetTemplate)]
struct Foo(usize, #[template] Handle<Image>);

#[derive(Event, EntityEvent)]
struct Explode;

#[derive(Component, Default, Clone)]
struct Thing;

fn print<C: Component + std::fmt::Debug>(
    query: Query<(Entity, Option<&Name>, Option<&ChildOf>, &C), Changed<C>>,
) {
    for (e, name, child_of, c) in &query {
        println!("Changed {e:?} {name:?} {child_of:?} {c:#?}");
    }
}

#[derive(Component, Debug, GetTemplate)]
enum Team {
    Red {
        x: usize,
        y: Nested,
    },
    Blue,
    #[default]
    Green(usize, usize),
}

#[derive(Component, GetTemplate)]
enum Blah {
    #[default]
    A(Hi),
    B(Arrrrg),
    C(String),
}

#[derive(Default)]
struct Arrrrg(Option<Box<dyn Fn(&mut World) + Send + Sync>>);

impl Clone for Arrrrg {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl<F: Fn(&mut World) + Send + Sync> From<F> for Arrrrg {
    fn from(value: F) -> Self {
        todo!()
    }
}

#[derive(Clone, Default)]
struct Hi {
    size: usize,
}

fn test() -> impl Scene {
    bsn! {
        Blah::A(Hi {size: 10})
        Blah::B(|world: &mut World| {})
        Blah::C("hi")
    }
}

fn widget(children: impl SceneList) -> impl Scene {
    bsn! {
        Node {
            width: Val::Px(1.0)
        } [
            {children}
        ]
    }
}
