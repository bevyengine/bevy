use std::{mem::drop, process::exit, thread::sleep, time::Duration};

use bevy::{
    animation::{stage::ANIMATE, Animator, Clip},
    asset::{AssetServerSettings, LoadState},
    ecs::Schedule,
    prelude::*,
    render::{pipeline::RenderPipelines, render_graph::base::MainPass},
};

fn main() {
    let mut bench = bevy::animation::Bench::build(None);
    bench
        .builder
        // To load
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::gltf::GltfPlugin::default())
        // These are need to load the GLTF
        .add_asset::<Mesh>()
        .register_type::<Draw>()
        .register_type::<MainPass>()
        .register_type::<Visible>()
        .register_type::<RenderPipelines>()
        .add_asset::<StandardMaterial>();

    bench.warm();

    let app = &mut bench.builder.app;
    let asset_server = app.resources.get::<AssetServer>().unwrap();
    let scene: Handle<Scene> = asset_server.load("models/character_medium/character_medium.gltf");
    let idle: Handle<Clip> = asset_server.load("models/character_medium/idle.gltf#Anim0");
    let run: Handle<Clip> = asset_server.load("models/character_medium/run.gltf#Anim0");
    drop(asset_server);

    let t = Duration::from_millis(50);
    for h in &[
        scene.clone_untyped(),
        idle.clone_untyped(),
        run.clone_untyped(),
    ] {
        loop {
            app.update();

            let asset_server = app.resources.get::<AssetServer>().unwrap();
            let state = asset_server.get_load_state(h);
            drop(asset_server);

            // println!("{:?}", state);
            if state == LoadState::Loaded {
                break;
            }
            sleep(t);
        }
    }

    let mut spawner = app.resources.get_mut::<SceneSpawner>().unwrap();
    spawner.spawn(scene);
    drop(spawner);
    app.update();

    let mut ok = false;
    for (mut animator,) in app.world.query_mut::<(&mut Animator,)>() {
        animator.add_layer(idle.clone(), 0.5);
        animator.add_layer(run.clone(), 0.5);
        ok = true;
    }
    assert!(ok, "scene not spawned or doesn't contain an animator");

    bench.run(100_000);
}
