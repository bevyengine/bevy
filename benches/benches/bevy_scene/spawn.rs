use criterion::{criterion_group, Criterion};
use std::time::Duration;

use bevy_app::App;
use bevy_asset::{AssetServer, Assets};
use bevy_ecs::prelude::*;
use bevy_scene2::{prelude::*, ScenePatch};
use bevy_ui::prelude::*;

criterion_group!(benches, spawn);

fn ui() -> impl Scene {
    bsn! {
        Node
        Children [
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
            (:button Node {width: Val::Px(200.) }),
        ]
    }
}

fn ui_loaded_asset() -> impl Scene {
    bsn! {
        Node
        Children [
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
            (:"scene://button.bsn" Node {width: Val::Px(200.) }),
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
        app.add_plugins((
            bevy_app::TaskPoolPlugin::default(),
            bevy_asset::AssetPlugin::default(),
            bevy_scene2::ScenePlugin,
        ));
        app.finish();
        app.cleanup();

        let assets = app.world().resource::<AssetServer>();
        let handle =
            assets.load_with_path("scene://button.bsn", ScenePatch::load(assets, button()));
        assert!(app.world().get_resource::<Assets<ScenePatch>>().is_some());

        app.update();

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
