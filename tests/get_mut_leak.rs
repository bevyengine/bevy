//! Tests if touching mutably a asset that gets extracted to the render world
//! causes a leak

use std::time::Duration;

use bevy::{
    app::{App, PluginGroup, Startup, Update},
    asset::{Asset, Assets, Handle},
    audio::AudioPlugin,
    color::Color,
    diagnostic::{DiagnosticsStore, LogDiagnosticsPlugin},
    ecs::system::{Commands, Local, Res, ResMut},
    math::primitives::Sphere,
    pbr::{MeshMaterial3d, StandardMaterial},
    render::{
        diagnostic::{MeshAllocatorDiagnosticPlugin, RenderAssetDiagnosticPlugin},
        mesh::{Mesh, Mesh3d, Meshable, RenderMesh},
    },
    window::WindowPlugin,
    winit::WinitPlugin,
    DefaultPlugins,
};

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

fn mesh_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_leaker: Local<Vec<Handle<Mesh>>>,
    mut material_leaker: Local<Vec<Handle<StandardMaterial>>>,
) {
    bevy::log::info!("Mesh setup");
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.).mesh().ico(79).unwrap())),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    for _ in 0..16 {
        mesh_leaker.push(meshes.add(Sphere::new(1.).mesh().ico(79).unwrap()));
    }
    for _ in 0..1000 {
        material_leaker.push(materials.add(Color::WHITE));
    }
}

fn touch_mutably<A: Asset>(mut assets: ResMut<Assets<A>>) {
    for _ in assets.iter_mut() {}
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
            render_meshes.value < allocations.value * 10.,
            "Detected leak"
        );
    }
}
