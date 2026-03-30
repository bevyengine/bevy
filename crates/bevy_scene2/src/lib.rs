//! Provides BSN functionality. See [`Scene`], [`SceneList`], [`ScenePatch`], and the [`bsn!`] / [`bsn_list!`] macros for more information.

/// The Bevy Scene prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::{
        bsn, bsn_list, on, CommandsSceneExt, EntityCommandsSceneExt, EntityWorldMutSceneExt,
        PatchFromTemplate, PatchTemplate, Scene, SceneList, ScenePatchInstance, WorldSceneExt,
    };
}

/// Functionality used by the [`bsn!`] macro.
pub mod macro_utils;

extern crate alloc;

mod resolved_scene;
mod scene;
mod scene_list;
mod scene_patch;
mod spawn;

pub use bevy_scene2_macros::*;
pub use resolved_scene::*;
pub use scene::*;
pub use scene_list::*;
pub use scene_patch::*;
pub use spawn::*;

use bevy_app::{App, Plugin, SceneSpawnerSystems, SpawnScene};
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;

/// Adds support for spawning Bevy Scenes. See [`Scene`], [`SceneList`], [`ScenePatch`], and the [`bsn!`] macro for more information.
#[derive(Default)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QueuedScenes>()
            .init_asset::<ScenePatch>()
            .init_asset::<SceneListPatch>()
            .add_systems(
                SpawnScene,
                (resolve_scene_patches, spawn_queued)
                    .chain()
                    .in_set(SceneSpawnerSystems::Scene2Spawn)
                    .after(SceneSpawnerSystems::SceneSpawn),
            )
            .add_observer(on_add_scene_patch_instance);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::{self as bevy_scene2, ScenePlugin};
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{Asset, AssetApp, AssetPlugin, AssetServer, Handle};
    use bevy_ecs::prelude::*;
    use bevy_reflect::TypePath;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));
        app
    }

    #[test]
    fn inheritance_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn b() -> impl Scene {
            bsn! {
                :a
                Position { x: 1. }
                Children [ #Y ]
            }
        }

        fn a() -> impl Scene {
            bsn! {
                Position { y: 2. }
                Children [ #X ]
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 1.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let y = world.entity(children[1]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn inline_scene_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Position { x: 1. }
                Children [ #Y ]
            }
        }

        fn a() -> impl Scene {
            bsn! {
                Position { y: 2. }
                Children [ #X ]
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 1.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let y = world.entity(children[1]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn hierarchy() {
        let mut app = test_app();
        let world = app.world_mut();

        fn scene() -> impl Scene {
            bsn! {
                #A
                Children [
                    (
                        #B
                        Children [
                            #X
                        ]
                    ),
                    (
                        #C
                        Children [
                            #Y
                        ]
                    )
                ]
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();

        let a = world.entity(id);
        let name = a.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "A");

        let children = a.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let b = world.entity(children[0]);
        let c = world.entity(children[1]);

        let name = b.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "B");

        let name = c.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "C");

        let children = b.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);
        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let children = c.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);
        let y = world.entity(children[0]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn constant_values() {
        let mut app = test_app();
        let world = app.world_mut();

        const X_AXIS: usize = 1;
        const XAXIS: usize = 2;

        #[derive(Component, FromTemplate)]
        struct Value(usize);

        fn x_axis() -> impl Scene {
            bsn! {Value(X_AXIS)}
        }

        fn xaxis() -> impl Scene {
            bsn! {Value(XAXIS)}
        }

        let entity = world.spawn_scene(x_axis()).unwrap();
        assert_eq!(entity.get::<Value>().unwrap().0, 1);

        let entity = world.spawn_scene(xaxis()).unwrap();
        assert_eq!(entity.get::<Value>().unwrap().0, 2);
    }

    #[derive(Component, FromTemplate)]
    struct Reference(Entity);

    #[test]
    fn bsn_name_references() {
        let mut app = test_app();
        let world = app.world_mut();

        fn a() -> impl Scene {
            bsn! {
                #X
                Children [
                    (:b Reference(#X))
                ]
            }
        }

        fn b() -> impl Scene {
            let inline = bsn! {#Y Reference(#Y) Children [ Reference(#Y)] };
            bsn! {
                #X
                Children [
                    Reference(#X),
                    (inline Reference(#X)),
                ]
            }
        }

        let id = world.spawn_scene(a()).unwrap().id();

        let a = world.entity(id);
        let name = a.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let children = a.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);

        let b = world.entity(children[0]);
        let reference = b.get::<Reference>().unwrap();
        assert_eq!(reference.0, id);

        let b_name = b.get::<Name>().unwrap();
        assert_eq!(b_name.as_str(), "X");

        let grandchildren = b.get::<Children>().unwrap();
        assert_eq!(grandchildren.len(), 2);

        let grandchild = world.entity(grandchildren[0]);
        assert_eq!(grandchild.get::<Reference>().unwrap().0, b.id());

        let grandchild = world.entity(grandchildren[1]);
        assert_eq!(grandchild.get::<Reference>().unwrap().0, b.id());
        assert_eq!(grandchild.get::<Name>().unwrap().as_str(), "Y");

        assert_eq!(
            grandchild.id(),
            world
                .entity(grandchild.get::<Children>().unwrap()[0])
                .get::<Reference>()
                .unwrap()
                .0
        );
    }

    #[test]
    fn bsn_list_name_references() {
        let mut app = test_app();
        let world = app.world_mut();

        fn b() -> impl Scene {
            bsn! {
                #Z
                Children [
                    Reference(#Z)
                ]
            }
        }

        fn a() -> impl SceneList {
            bsn_list![
                (
                    #X
                    Reference(#Y)
                    Children [
                        (#Z Reference(#X))
                    ]

                ),
                (
                    #Y
                    Reference(#X)
                    Children [
                        Reference(#Y)
                    ]
                ),
                (:b #Z)
            ]
        }

        let ids = world.spawn_scene_list(a()).unwrap();
        assert_eq!(ids.len(), 3);

        let e0 = world.entity(ids[0]);
        let name = e0.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");
        let reference = e0.get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[1]);

        let child0 = e0.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[0]);

        let e1 = world.entity(ids[1]);
        let name = e1.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");

        let reference = e1.get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[0]);

        let child0 = e1.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[1]);

        let e2 = world.entity(ids[2]);
        let name = e2.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Z");
        let child0 = e2.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[2]);
    }

    #[test]
    fn on_template() {
        #[derive(Resource)]
        struct Exploded(Option<Entity>);

        #[derive(EntityEvent)]
        struct Explode(Entity);

        let mut app = test_app();
        let world = app.world_mut();
        world.insert_resource(Exploded(None));

        fn scene() -> impl Scene {
            bsn! {
                on(|explode: On<Explode>, mut exploded: ResMut<Exploded>|{
                    exploded.0 = Some(explode.0);
                })
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();
        world.trigger(Explode(id));
        let exploded = world.resource::<Exploded>();
        assert_eq!(exploded.0, Some(id));
    }

    #[test]
    fn enum_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        enum Foo {
            #[default]
            Bar {
                x: u32,
                y: u32,
                z: u32,
            },
            Baz(usize),
            Qux,
        }

        fn a() -> impl Scene {
            bsn! {
                Foo::Baz(10)
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo::Bar { x: 1 }
            }
        }

        fn c() -> impl Scene {
            bsn! {
                b()
                Foo::Bar { y: 2 }
            }
        }

        fn d() -> impl Scene {
            bsn! {
                c()
                Foo::Qux
            }
        }

        let id = world.spawn_scene(c()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Bar { x: 1, y: 2, z: 0 }, *foo);

        let id = world.spawn_scene(a()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Baz(10), *foo);

        let id = world.spawn_scene(d()).unwrap().id();
        let root = world.entity(id);
        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Qux, *foo);
    }

    #[test]
    fn struct_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Foo {
            x: u32,
            y: u32,
            z: u32,
            nested: Bar,
        }

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Bar(usize, usize, usize);

        fn a() -> impl Scene {
            bsn! {
                Foo {
                    x: 1,
                    nested: Bar(1, 1),
                }
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo {
                    y: 2,
                    nested: Bar(2),
                }
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(
            *foo,
            Foo {
                x: 1,
                y: 2,
                z: 0,
                nested: Bar(2, 1, 0)
            }
        );
    }

    #[test]
    fn handle_template() {
        let mut app = test_app();
        app.init_asset::<Image>();

        #[derive(Asset, TypePath)]
        struct Image;

        let handle = app.world().resource::<AssetServer>().load("image.png");
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Sprite(Handle<Image>);

        fn scene() -> impl Scene {
            bsn! {
                Sprite("image.png")
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();
        let root = world.entity(id);

        let sprite = root.get::<Sprite>().unwrap();
        assert_eq!(sprite.0, handle);
    }

    #[test]
    fn scene_list_children() {
        let mut app = test_app();
        let world = app.world_mut();

        fn root(children: impl SceneList) -> impl Scene {
            bsn! {
                Children [
                    #A,
                    {children},
                    #D
                ]
            }
        }

        let children = bsn_list! [
            #B,
            #C,
        ];

        let id = world.spawn_scene(root(children)).unwrap().id();
        let root = world.entity(id);
        let children = root.get::<Children>().unwrap();
        let a = world.entity(children[0]).get::<Name>().unwrap();
        let b = world.entity(children[1]).get::<Name>().unwrap();
        let c = world.entity(children[2]).get::<Name>().unwrap();
        let d = world.entity(children[3]).get::<Name>().unwrap();
        assert_eq!(a.as_str(), "A");
        assert_eq!(b.as_str(), "B");
        assert_eq!(c.as_str(), "C");
        assert_eq!(d.as_str(), "D");
    }

    #[test]
    fn generic_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Foo<T: FromTemplate<Template: Default + Template<Output = T>>> {
            value: T,
            number: u32,
        }

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Position {
            x: u32,
            y: u32,
            z: u32,
        }

        fn a() -> impl Scene {
            bsn! {
                Foo::<Position> {
                    value: Position { x: 1 }
                }
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo::<Position> {
                    value: Position { y: 2 },
                    number: 10,
                }
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo<Position>>().unwrap();
        assert_eq!(
            *foo,
            Foo {
                value: Position { x: 1, y: 2, z: 0 },
                number: 10
            }
        );
    }

    #[test]
    fn empty_scene_expressions() {
        let mut app = test_app();
        let world = app.world_mut();
        fn a() -> impl Scene {
            bsn! {
                {}
            }
        }
        world.spawn_scene(a()).unwrap();
    }
}
