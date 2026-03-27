use bevy_reflect::TypePath;
use criterion::{criterion_group, Criterion};
use std::{path::Path, time::Duration};

use bevy_app::App;
use bevy_asset::{
    io::{
        memory::{Dir, MemoryAssetReader},
        AssetSourceBuilder, AssetSourceId,
    },
    AssetApp, AssetLoader, AssetServer, Assets,
};
use bevy_ecs::prelude::*;
use bevy_scene2::{prelude::*, ScenePatch};
use bevy_ui::prelude::*;

criterion_group!(benches, spawn);

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
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
            (Text("Text") Marker),
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
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
            (Text("Text".into()), Marker),
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

fn spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(4));
    group.bench_function("ui_immediate_function_scene", |b| {
        let mut app = App::new();
        app.add_plugins((bevy_asset::AssetPlugin::default(), bevy_scene2::ScenePlugin));

        b.iter(move || {
            app.world_mut().spawn_scene(ui()).unwrap();
        });
    });
    group.bench_function("ui_immediate_loaded_scene", |b| {
        let mut app = App::new();
        let dir = Dir::default();
        let dir_clone = dir.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: dir_clone.clone(),
                })
            }),
        );
        app.add_plugins((
            bevy_app::TaskPoolPlugin::default(),
            bevy_asset::AssetPlugin::default(),
            bevy_scene2::ScenePlugin,
        ));
        app.finish();
        app.cleanup();

        // Create a fake loader to act as a ScenePatch loaded from a file.
        app.register_asset_loader(FakeSceneLoader);

        #[derive(TypePath)]
        struct FakeSceneLoader;

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
                Ok(ScenePatch::load_with(load_context, button()))
            }
        }

        // Insert an asset that the fake loader can fake read.
        dir.insert_asset_text(Path::new("button.bsn"), "");

        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle = asset_server.load("button.bsn");
        assert!(app.world().get_resource::<Assets<ScenePatch>>().is_some());

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
        let mut app = App::new();
        app.add_plugins((bevy_asset::AssetPlugin::default(), bevy_scene2::ScenePlugin));

        b.iter(move || {
            app.world_mut().spawn(raw_ui());
        });
    });
    group.finish();
}
