//! This example illustrates how to wait for multiple assets to be loaded.

use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use bevy::{gltf::Gltf, prelude::*, tasks::AsyncComputeTaskPool};
use event_listener::Event;
use futures_lite::Future;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<LoadingState>()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .add_systems(Startup, setup_assets)
        .add_systems(Startup, setup_scene)
        .add_systems(Startup, setup_ui)
        // This showcases how to wait for assets using sync code.
        // This approach polls a value in a system.
        .add_systems(Update, wait_on_load.run_if(assets_loaded))
        // This showcases how to wait for assets using async
        // by spawning a `Future` in `AsyncComputeTaskPool`.
        .add_systems(
            Update,
            get_async_loading_state.run_if(in_state(LoadingState::Loading)),
        )
        // This showcases how to react to asynchronous world mutation synchronously.
        .add_systems(
            OnExit(LoadingState::Loading),
            despawn_loading_state_entities,
        )
        .run();
}

/// [`States`] of asset loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, States, Default)]
pub enum LoadingState {
    /// Is loading.
    #[default]
    Loading,
    /// Loading completed.
    Loaded,
}

/// Holds a bunch of [`Gltf`]s that takes time to load.
#[derive(Debug, Resource)]
pub struct OneHundredThings([Handle<Gltf>; 100]);

/// This is required to support both sync and async.
///
/// For sync only the easiest implementation is
/// [`Arc<()>`] and use [`Arc::strong_count`] for completion.
/// [`Arc<Atomic*>`] is a more robust alternative.
#[derive(Debug, Resource, Deref)]
pub struct AssetBarrier(Arc<AssetBarrierInner>);

/// This guard is to be acquired by [`AssetServer::load_acquire`]
/// and dropped once finished.
#[derive(Debug, Deref)]
pub struct AssetBarrierGuard(Arc<AssetBarrierInner>);

/// Tracks how many guards are remaining.
#[derive(Debug, Resource)]
pub struct AssetBarrierInner {
    count: AtomicU32,
    /// This can be omitted if async is not needed.
    notify: Event,
}

/// State of loading asynchronously.
#[derive(Debug, Resource)]
pub struct AsyncLoadingState(Arc<AtomicBool>);

/// Entities that are to be removed once loading finished
#[derive(Debug, Component)]
pub struct Loading;

/// Marker for the "Loading..." Text component.
#[derive(Debug, Component)]
pub struct LoadingText;

impl AssetBarrier {
    /// Create an [`AssetBarrier`] with a [`AssetBarrierGuard`].
    pub fn new() -> (AssetBarrier, AssetBarrierGuard) {
        let inner = Arc::new(AssetBarrierInner {
            count: AtomicU32::new(1),
            notify: Event::new(),
        });
        (AssetBarrier(inner.clone()), AssetBarrierGuard(inner))
    }

    /// Returns true if all [`AssetBarrierGuard`] is dropped.
    pub fn is_ready(&self) -> bool {
        self.count.load(Ordering::Acquire) == 0
    }

    /// Wait for all [`AssetBarrierGuard`]s to be dropped asynchronously.
    pub fn wait_async(&self) -> impl Future<Output = ()> + 'static {
        let shared = self.0.clone();
        async move {
            loop {
                // Acquire an event listener.
                let listener = shared.notify.listen();
                // If all barrier guards are dropped, return
                if shared.count.load(Ordering::Acquire) == 0 {
                    return;
                }
                // Wait for the last barrier guard to notify us
                listener.await;
            }
        }
    }
}

// Increment count on clone.
impl Clone for AssetBarrierGuard {
    fn clone(&self) -> Self {
        self.count.fetch_add(1, Ordering::AcqRel);
        AssetBarrierGuard(self.0.clone())
    }
}

// Decrement count on drop.
impl Drop for AssetBarrierGuard {
    fn drop(&mut self) {
        let prev = self.count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // Notify all listeners if count reaches 0.
            self.notify.notify(usize::MAX);
        }
    }
}

fn setup_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let (barrier, guard) = AssetBarrier::new();
    commands.insert_resource(OneHundredThings(std::array::from_fn(|i| match i % 5 {
        0 => asset_server.load_acquire("models/GolfBall/GolfBall.glb", guard.clone()),
        1 => asset_server.load_acquire("models/AlienCake/alien.glb", guard.clone()),
        2 => asset_server.load_acquire("models/AlienCake/cakeBirthday.glb", guard.clone()),
        3 => asset_server.load_acquire("models/FlightHelmet/FlightHelmet.gltf", guard.clone()),
        4 => asset_server.load_acquire("models/torus/torus.gltf", guard.clone()),
        _ => unreachable!(),
    })));
    let future = barrier.wait_async();
    commands.insert_resource(barrier);

    let loading_state = Arc::new(AtomicBool::new(false));
    commands.insert_resource(AsyncLoadingState(loading_state.clone()));

    // await the `AssetBarrierFuture`.
    AsyncComputeTaskPool::get()
        .spawn(async move {
            future.await;
            // Notify via `AsyncLoadingState`
            loading_state.store(true, Ordering::Release);
        })
        .detach();
}

fn setup_ui(mut commands: Commands) {
    // Display the result of async loading.
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::End,

                ..default()
            },
            ..default()
        })
        .with_children(|b| {
            b.spawn((
                TextBundle {
                    text: Text {
                        sections: vec![TextSection {
                            value: "Loading...".to_owned(),
                            style: TextStyle {
                                font_size: 64.0,
                                color: Color::BLACK,
                                ..Default::default()
                            },
                        }],
                        justify: JustifyText::Right,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                LoadingText,
            ));
        });
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(10.0, 10.0, 15.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    // Plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50000.0, 50000.0)),
            material: materials.add(Color::srgb(0.7, 0.2, 0.2)),
            ..default()
        },
        Loading,
    ));
}

// A run condition for all assets being loaded.
fn assets_loaded(barrier: Option<Res<AssetBarrier>>) -> bool {
    // If our barrier isn't ready, return early and wait another cycle
    barrier.map(|b| b.is_ready()) == Some(true)
}

// This showcases how to wait for assets using sync code and systems.
//
// This function only runs if `assets_loaded` returns true.
fn wait_on_load(
    mut commands: Commands,
    foxes: Res<OneHundredThings>,
    gltfs: Res<Assets<Gltf>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Change color of plane to green
    commands.spawn((PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(50000.0, 50000.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        transform: Transform::from_translation(Vec3::Z * -0.01),
        ..default()
    },));

    // Spawn our scenes.
    for i in 0..10 {
        for j in 0..10 {
            let index = i * 10 + j;
            let position = Vec3::new(i as f32 - 5.0, 0.0, j as f32 - 5.0);
            // All gltfs must exist because this is guarded by the `AssetBarrier`.
            let gltf = gltfs.get(&foxes.0[index]).unwrap();
            let scene = gltf.scenes.first().unwrap().clone();
            commands.spawn(SceneBundle {
                scene,
                transform: Transform::from_translation(position),
                ..Default::default()
            });
        }
    }
}

// This showcases how to wait for assets using async.
fn get_async_loading_state(
    state: Res<AsyncLoadingState>,
    mut next_loading_state: ResMut<NextState<LoadingState>>,
    mut text: Query<&mut Text, With<LoadingText>>,
) {
    // Load the value written by the `Future`.
    let is_loaded = state.0.load(Ordering::Acquire);

    // If loaded, change the state.
    if is_loaded {
        next_loading_state.set(LoadingState::Loaded);
        if let Ok(mut text) = text.get_single_mut() {
            "Loaded!".clone_into(&mut text.sections[0].value);
        }
    }
}

// This showcases how to react to asynchronous world mutations synchronously.
fn despawn_loading_state_entities(mut commands: Commands, loading: Query<Entity, With<Loading>>) {
    // Despawn entities in the loading phase.
    for entity in loading.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Despawn resources used in the loading phase.
    commands.remove_resource::<AssetBarrier>();
    commands.remove_resource::<AsyncLoadingState>();
}
