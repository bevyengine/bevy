//! Integration tests for the `world.inspect*` BRP methods, run through the real HTTP server
//! and client rather than by calling the handlers directly.

#![cfg(all(feature = "http", feature = "client"))]
#![expect(clippy::std_instead_of_alloc, reason = "this is a std test binary")]

use bevy_app::{App, TaskPoolPlugin};
use bevy_ecs::{
    component::Component,
    name::Name,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
};
use bevy_reflect::Reflect;
use bevy_remote::http::RemoteHttpPlugin;
use bevy_remote::{
    client::{BrpClient, BrpClientError},
    error_codes, RemotePlugin,
};
use bevy_tasks::futures_lite::future;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::sync::Arc;

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Health(u32);

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
struct Score(u32);

fn health_type_path() -> String {
    core::any::type_name::<Health>().to_string()
}

fn score_type_path() -> String {
    core::any::type_name::<Score>().to_string()
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

struct TestServer {
    client: BrpClient,
    entity_bits: u64,
    despawned_entity_bits: u64,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl TestServer {
    fn spawn() -> Self {
        let port = free_port();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let (sender, receiver) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let mut app = App::new();
            app.add_plugins((
                TaskPoolPlugin::default(),
                RemotePlugin::default(),
                RemoteHttpPlugin::default().with_port(port),
            ));

            {
                let registry = app.world().resource::<AppTypeRegistry>();
                let mut registry = registry.write();
                registry.register::<Health>();
                registry.register::<Score>();
            }
            app.world_mut().insert_resource(Score(11));

            let entity_bits = app
                .world_mut()
                .spawn((Name::new("Player"), Health(7)))
                .id()
                .to_bits();
            let despawned_entity = app.world_mut().spawn(Health(1)).id();
            let despawned_entity_bits = despawned_entity.to_bits();
            app.world_mut().despawn(despawned_entity);

            app.finish();
            app.cleanup();

            let _ = sender.send((entity_bits, despawned_entity_bits));

            while !thread_stop.load(Ordering::Relaxed) {
                app.update();
                std::thread::sleep(Duration::from_millis(5));
            }
        });

        let (entity_bits, despawned_entity_bits) = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("the server thread should report the spawned entities");

        Self {
            client: BrpClient::localhost(port),
            entity_bits,
            despawned_entity_bits,
            stop,
            handle: Some(handle),
        }
    }

    fn call(&self, method: &str, params: Option<Value>) -> Result<Value, BrpClientError> {
        future::block_on(async {
            let mut last = self.client.call(method, params.clone()).await;
            for _ in 0..200 {
                if last.is_ok() {
                    return last;
                }
                std::thread::sleep(Duration::from_millis(10));
                last = self.client.call(method, params.clone()).await;
            }
            last
        })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn inspect_returns_label_and_component() {
    let server = TestServer::spawn();

    let result = server
        .call(
            "world.inspect",
            Some(json!({ "entity": server.entity_bits })),
        )
        .expect("world.inspect should succeed");

    assert_eq!(result["label"]["label"], json!("Player"));
    let components = result["components"].as_array().unwrap();
    let health = components
        .iter()
        .find(|component| component["name"] == json!(health_type_path()))
        .expect("the Health component should be present");
    assert_eq!(health["serialized_value"], json!(7));
}

#[test]
fn inspect_missing_entity_returns_not_found() {
    let server = TestServer::spawn();

    let error = server
        .call(
            "world.inspect",
            Some(json!({ "entity": server.despawned_entity_bits })),
        )
        .expect_err("a despawned entity should not be inspectable");

    match error {
        BrpClientError::Remote(error) => assert_eq!(error.code, error_codes::ENTITY_NOT_FOUND),
        other => panic!("expected a remote error, got {other:?}"),
    }
}

#[test]
fn inspect_component_by_type_path() {
    let server = TestServer::spawn();

    let result = server
        .call(
            "world.inspect_component",
            Some(json!({ "entity": server.entity_bits, "component": health_type_path() })),
        )
        .expect("world.inspect_component should succeed");

    assert_eq!(result["serialized_value"], json!(7));
}

#[test]
fn inspect_component_type_counts_entities() {
    let server = TestServer::spawn();

    let result = server
        .call(
            "world.inspect_component_type",
            Some(json!({ "component": health_type_path() })),
        )
        .expect("world.inspect_component_type should succeed");

    assert!(result["entity_count"].as_u64().unwrap() >= 1);
}

#[test]
fn inspect_resource_returns_serialized_value() {
    let server = TestServer::spawn();

    let result = server
        .call(
            "world.inspect_resource",
            Some(json!({ "resource": score_type_path() })),
        )
        .expect("world.inspect_resource should succeed");

    assert_eq!(result["serialized_value"], json!(11));
}

#[test]
fn inspect_all_resources_includes_inserted_resource() {
    let server = TestServer::spawn();

    let result = server
        .call("world.inspect_all_resources", None)
        .expect("world.inspect_all_resources should succeed");

    assert!(result
        .as_array()
        .unwrap()
        .iter()
        .any(|inspection| inspection["name"] == json!(score_type_path())));
}

#[test]
fn summarize_reports_entity_count() {
    let server = TestServer::spawn();

    let result = server
        .call("world.summarize", None)
        .expect("world.summarize should succeed");

    assert!(result["total_entities"].as_u64().unwrap() >= 1);
}

#[test]
fn component_metadata_contains_health() {
    let server = TestServer::spawn();

    let result = server
        .call("registry.component_metadata", None)
        .expect("registry.component_metadata should succeed");

    let map = result["map"].as_object().unwrap();
    assert!(map
        .values()
        .any(|metadata| metadata["name"] == json!(health_type_path())));
}

#[test]
fn inspect_unknown_component_name_errors() {
    let server = TestServer::spawn();

    let error = server
        .call(
            "world.inspect_component_type",
            Some(json!({ "component": "not::a::Component" })),
        )
        .expect_err("an unknown component name should be rejected");

    match error {
        BrpClientError::Remote(error) => {
            assert_eq!(error.code, error_codes::COMPONENT_NAME_NOT_IN_METADATA);
        }
        other => panic!("expected a remote error, got {other:?}"),
    }
}
