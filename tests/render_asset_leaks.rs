//! Tests if touching mutably a asset that gets extracted to the render world
//! causes a leak

use std::time::Duration;

use bevy::{
    app::{App, PluginGroup, Startup, Update},
    asset::{Asset, Assets, Handle},
    audio::AudioPlugin,
    color::Color,
    diagnostic::{DiagnosticsStore, LogDiagnosticsPlugin},
    ecs::system::{Res, ResMut},
    math::primitives::Sphere,
    pbr::{
        diagnostic::MaterialAllocatorDiagnosticPlugin, Material, PreparedMaterial, StandardMaterial,
    },
    render::{
        diagnostic::{MeshAllocatorDiagnosticPlugin, RenderAssetDiagnosticPlugin},
        mesh::{Mesh, Meshable, RenderMesh},
    },
    window::WindowPlugin,
    winit::WinitPlugin,
    DefaultPlugins,
};
use bevy_ecs::{resource::Resource, system::Commands};

#[test]
fn check_mesh_leak() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .build()
            .disable::<AudioPlugin>()
            .disable::<WinitPlugin>()
            .disable::<WindowPlugin>(),
        LogDiagnosticsPlugin {
            wait_duration: Duration::ZERO,
            ..Default::default()
        },
        RenderAssetDiagnosticPlugin::<RenderMesh>::new(" meshes"),
        MeshAllocatorDiagnosticPlugin,
    ))
    .add_systems(Startup, mesh_setup)
    .add_systems(
        Update,
        (touch_mutably::<Mesh>, crash_on_mesh_leak_detection),
    );

    app.finish();
    app.cleanup();

    for _ in 0..100 {
        app.update();
    }
}

#[ignore = "FIXME Failing test"]
#[test]
fn check_standard_material_leak() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .build()
            .disable::<AudioPlugin>()
            .disable::<WinitPlugin>()
            .disable::<WindowPlugin>(),
        LogDiagnosticsPlugin {
            wait_duration: Duration::ZERO,
            ..Default::default()
        },
        RenderAssetDiagnosticPlugin::<PreparedMaterial<StandardMaterial>>::new(" materials"),
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
    ))
    .add_systems(Startup, mesh_setup)
    .add_systems(
        Update,
        (
            touch_mutably::<StandardMaterial>,
            crash_on_material_leak_detection::<StandardMaterial>,
        ),
    );

    app.finish();
    app.cleanup();

    for _ in 0..100 {
        app.update();
    }
}

#[test]
fn check_mesh_churn_leak() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .build()
            .disable::<AudioPlugin>()
            .disable::<WinitPlugin>()
            .disable::<WindowPlugin>(),
        LogDiagnosticsPlugin {
            wait_duration: Duration::ZERO,
            ..Default::default()
        },
        RenderAssetDiagnosticPlugin::<RenderMesh>::new(" meshes"),
        MeshAllocatorDiagnosticPlugin,
    ))
    .add_systems(Startup, mesh_setup)
    .add_systems(Update, (churn::<Mesh>, crash_on_mesh_leak_detection));

    app.finish();
    app.cleanup();

    for _ in 0..100 {
        app.update();
    }
}

#[ignore = "FIXME Failing test"]
#[test]
fn check_standard_material_churn_leak() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .build()
            .disable::<AudioPlugin>()
            .disable::<WinitPlugin>()
            .disable::<WindowPlugin>(),
        LogDiagnosticsPlugin {
            wait_duration: Duration::ZERO,
            ..Default::default()
        },
        RenderAssetDiagnosticPlugin::<PreparedMaterial<StandardMaterial>>::new(" materials"),
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
    ))
    .add_systems(Startup, mesh_setup)
    .add_systems(
        Update,
        (
            churn::<StandardMaterial>,
            crash_on_material_leak_detection::<StandardMaterial>,
        ),
    );

    app.finish();
    app.cleanup();

    for _ in 0..100 {
        app.update();
    }
}

#[derive(Resource)]
struct Leaker<A: Asset>(Vec<Handle<A>>);

fn mesh_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    bevy::log::info!("Mesh setup");

    let mut mesh_leaker = Vec::with_capacity(16);
    for _ in 0..16 {
        mesh_leaker.push(meshes.add(Sphere::new(1.).mesh().ico(79).unwrap()));
    }
    commands.insert_resource(Leaker(mesh_leaker));
    let mut material_leaker = Vec::with_capacity(1000);
    for _ in 0..1000 {
        material_leaker.push(materials.add(Color::WHITE));
    }
    commands.insert_resource(Leaker(material_leaker));
}

fn touch_mutably<A: Asset>(mut assets: ResMut<Assets<A>>) {
    for _ in assets.iter_mut() {}
}

fn churn<A: Asset>(mut assets: ResMut<Assets<A>>, mut leaker: ResMut<Leaker<A>>) {
    let all_ids = leaker.0.drain(..).collect::<Vec<_>>();
    for id in all_ids {
        let asset = assets.remove(id.id()).unwrap();
        leaker.0.push(assets.add(asset));
    }
}

fn crash_on_mesh_leak_detection(diagnostic_store: Res<DiagnosticsStore>) {
    if let (Some(render_meshes), Some(allocations)) = (
        diagnostic_store
            .get_measurement(
                &RenderAssetDiagnosticPlugin::<RenderMesh>::render_asset_diagnostic_path(),
            )
            .filter(|diag| diag.value > 0.),
        diagnostic_store
            .get_measurement(MeshAllocatorDiagnosticPlugin::allocations_diagnostic_path())
            .filter(|diag| diag.value > 0.),
    ) {
        assert!(
            allocations.value < render_meshes.value * 10.,
            "Detected leak"
        );
    }
}

fn crash_on_material_leak_detection<M: Material>(diagnostic_store: Res<DiagnosticsStore>) {
    if let (Some(materials), Some(allocations)) = (
        diagnostic_store
            .get_measurement(
                &RenderAssetDiagnosticPlugin::<PreparedMaterial<M>>::render_asset_diagnostic_path(),
            )
            .filter(|diag| diag.value > 0.),
        diagnostic_store
            .get_measurement(&MaterialAllocatorDiagnosticPlugin::<M>::allocations_diagnostic_path())
            .filter(|diag| diag.value > 0.),
    ) {
        assert!(allocations.value < materials.value * 10., "Detected leak");
    }
}
