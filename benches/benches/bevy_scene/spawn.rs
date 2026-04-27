use bevy_reflect::TypePath;
use criterion::{criterion_group, Criterion};
use glam::Mat4;
use std::{path::Path, time::Duration};

use bevy_app::App;
use bevy_asset::{
    asset_value,
    io::{
        memory::{Dir, MemoryAssetReader},
        AssetSourceBuilder, AssetSourceId,
    },
    Asset, AssetApp, AssetLoader, AssetServer, Assets, Handle,
};
use bevy_ecs::prelude::*;
use bevy_scene::{prelude::*, ScenePatch};
use bevy_ui::prelude::*;

criterion_group!(benches, spawn);

fn spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(4));
    group.bench_function("ui_immediate_function_scene", |b| {
        let mut app = bench_app(|_| {}, |_| {});
        b.iter(move || {
            app.world_mut().spawn_scene(ui()).unwrap();
        });
    });
    group.bench_function("ui_immediate_loaded_scene", |b| {
        let dir = Dir::default();
        let mut app = bench_app(
            |app| {
                in_memory_asset_source(dir.clone(), app);
            },
            |app| {
                app.register_asset_loader(FakeSceneLoader::new(button));
            },
        );

        // Insert an asset that the fake loader can fake read.
        dir.insert_asset_text(Path::new("button.bsn"), "");

        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle = asset_server.load("button.bsn");

        run_app_until(&mut app, || asset_server.is_loaded(&handle));

        let patch = app
            .world()
            .resource::<Assets<ScenePatch>>()
            .get(&handle)
            .unwrap();
        assert!(patch.resolved.is_some());

        b.iter(move || {
            app.world_mut().spawn_scene(ui_loaded_asset()).unwrap();
        });

        drop(handle);
    });
    group.bench_function("ui_raw_bundle_no_scene", |b| {
        let mut app = bench_app(|_| {}, |_| {});

        b.iter(move || {
            app.world_mut().spawn(raw_ui());
        });
    });

    group.bench_function("handle_template_handle", |b| {
        let dir = Dir::default();
        let mut app = bench_app(
            |app| {
                in_memory_asset_source(dir.clone(), app);
            },
            |app| {
                app.init_asset::<EmptyAsset>();
                let assets = app.world().resource::<AssetServer>();
                let handles = (0..10).map(|_| assets.add(EmptyAsset)).collect::<Vec<_>>();
                app.register_asset_loader(FakeSceneLoader::new(move || {
                    asset_handle_scene(handles.clone())
                }));
            },
        );

        dir.insert_asset_text(Path::new("a.bsn"), "");

        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle = asset_server.load::<ScenePatch>("a.bsn");

        run_app_until(&mut app, || asset_server.is_loaded(&handle));

        let world = app.world_mut();
        b.iter(|| {
            for _ in 0..100 {
                world.spawn_scene(bsn! { :"a.bsn" }).unwrap();
            }
        });
    });

    group.bench_function("handle_template_value", |b| {
        let dir = Dir::default();
        let mut app = bench_app(
            |app| {
                in_memory_asset_source(dir.clone(), app);
            },
            |app| {
                app.register_asset_loader(FakeSceneLoader::new(asset_value_scene));
                app.init_asset::<EmptyAsset>();
            },
        );

        dir.insert_asset_text(Path::new("a.bsn"), "");

        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle = asset_server.load::<ScenePatch>("a.bsn");

        run_app_until(&mut app, || asset_server.is_loaded(&handle));

        let world = app.world_mut();
        b.iter(|| {
            for _ in 0..100 {
                world.spawn_scene(bsn! { :"a.bsn" }).unwrap();
            }
        });
    });
    group.finish();
}

#[derive(Asset, TypePath)]
struct EmptyAsset;

#[derive(Component, FromTemplate)]
#[expect(unused, reason = "this is just used for init")]
struct AssetReference(Handle<EmptyAsset>);

fn asset_value_scene() -> impl Scene {
    let children = (0..10)
        .map(|_| {
            bsn! {AssetReference(asset_value(EmptyAsset))}
        })
        .collect::<Vec<_>>();
    bsn! {
        Children [{children}]
    }
}

fn asset_handle_scene(mut handles: Vec<Handle<EmptyAsset>>) -> impl Scene {
    let children = handles
        .drain(..)
        .map(|handle| {
            bsn! {AssetReference({handle.clone()})}
        })
        .collect::<Vec<_>>();
    bsn! {
        Children [{children}]
    }
}

fn ui() -> impl Scene {
    bsn! {
        Node
        Children [
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
            (:button Node { width: Val::Px(200.) }),
        ]
    }
}

fn ui_loaded_asset() -> impl Scene {
    bsn! {
        Node
        Children [
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
            (:"button.bsn" Node { width: Val::Px(200.) }),
        ]
    }
}

// A non-Node component that we add to force archetype moves, inflating their cost if/when they happen
#[derive(Component, Default, Clone)]
struct Marker;

#[derive(Component, Default, Clone)]
#[expect(unused, reason = "this exists to take up space")]
struct Marker1(Mat4);
#[derive(Component, Default, Clone)]
#[expect(unused, reason = "this exists to take up space")]
struct Marker2(Mat4);
#[derive(Component, Default, Clone)]
#[expect(unused, reason = "this exists to take up space")]
struct Marker3(Mat4);

fn button() -> impl Scene {
    bsn! {
        Button
        Node {
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
            (Text("Text") Marker Marker1 Marker2 Marker3),
        ]
    }
}

fn raw_button() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(200.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        children![
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
            (
                Text("Text".into()),
                Marker,
                Marker1::default(),
                Marker2::default(),
                Marker3::default()
            ),
        ],
    )
}

fn raw_ui() -> impl Bundle {
    (
        Node::default(),
        children![
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
            raw_button(),
        ],
    )
}

/// Fork of `bevy_asset::tests::run_app_until`.
fn run_app_until(app: &mut App, mut predicate: impl FnMut() -> bool) {
    const LARGE_ITERATION_COUNT: usize = 10000;
    for _ in 0..LARGE_ITERATION_COUNT {
        app.update();
        if predicate() {
            return;
        }
    }

    panic!("Ran out of loops to return `Some` from `predicate`");
}

fn bench_app(before: impl FnOnce(&mut App), after: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    before(&mut app);
    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        bevy_asset::AssetPlugin::default(),
        bevy_scene::ScenePlugin,
    ));
    after(&mut app);
    app.finish();
    app.cleanup();
    app
}

fn in_memory_asset_source(dir: Dir, app: &mut App) {
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: dir.clone() })),
    );
}

#[derive(TypePath)]
struct FakeSceneLoader(Box<dyn Fn() -> Box<dyn Scene> + Send + Sync>);

impl FakeSceneLoader {
    pub fn new<S: Scene>(scene_fn: impl (Fn() -> S) + Send + Sync + 'static) -> Self {
        Self(Box::new(move || Box::new(scene_fn())))
    }
}

impl AssetLoader for FakeSceneLoader {
    type Asset = ScenePatch;
    type Error = std::io::Error;
    type Settings = ();

    async fn load(
        &self,
        _reader: &mut dyn bevy_asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        Ok(ScenePatch::load_with(load_context, (self.0)()))
    }
}
