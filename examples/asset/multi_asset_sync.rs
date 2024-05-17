//! This example illustrates how to wait for multiple assets to be loaded.

use std::{
    f32::consts::PI,
    pin::Pin,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
};

use bevy::{gltf::Gltf, prelude::*, tasks::AsyncComputeTaskPool};
use futures_lite::Future;

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
    /// This is not optimal, preferably use `event_listener::Event`
    wakers: Mutex<Vec<Waker>>,
}

/// Future for [`AssetBarrier`] completion.
#[must_use = "`Future`s do nothing unless polled."]
#[derive(Debug, Resource, Deref)]
pub struct AssetBarrierFuture(Arc<AssetBarrierInner>);

impl Future for AssetBarrierFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.count.load(Ordering::Acquire) == 0 {
            Poll::Ready(())
        } else {
            self.wakers.lock().unwrap().push(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// State of loading asynchronously.
#[derive(Debug, Resource)]
pub struct AsyncLoadingState(Arc<Mutex<String>>);

/// Entities that are to be removed once loading finished
#[derive(Debug, Component)]
pub struct Loading;

impl AssetBarrier {
    /// Create an [`AssetBarrier`] with a [`AssetBarrierGuard`].
    pub fn new() -> (AssetBarrier, AssetBarrierGuard) {
        let inner = Arc::new(AssetBarrierInner {
            count: AtomicU32::new(1),
            wakers: Mutex::default(),
        });
        (AssetBarrier(inner.clone()), AssetBarrierGuard(inner))
    }

    /// Returns true if all [`AssetBarrierGuard`] is dropped.
    pub fn is_ready(&self) -> bool {
        self.count.load(Ordering::Acquire) == 0
    }

    /// Wait for all [`AssetBarrierGuard`]s to be dropped asynchronously.
    pub fn wait_async(&self) -> AssetBarrierFuture {
        AssetBarrierFuture(self.0.clone())
    }
}

impl Clone for AssetBarrierGuard {
    fn clone(&self) -> Self {
        self.count.fetch_add(1, Ordering::AcqRel);
        AssetBarrierGuard(self.0.clone())
    }
}

impl Drop for AssetBarrierGuard {
    fn drop(&mut self) {
        let prev = self.count.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            self.wakers.lock().unwrap().drain(..).for_each(|w| w.wake());
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, wait_on_load)
        .add_systems(Update, get_async_loading_state)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    let loading_state = Arc::new(Mutex::new("Loading..".to_owned()));
    commands.insert_resource(AsyncLoadingState(loading_state.clone()));

    // await the `AssetBarrierFuture`.
    AsyncComputeTaskPool::get()
        .spawn(async move {
            future.await;
            "Loading Complete!".clone_into(&mut loading_state.lock().unwrap());
        })
        .detach();

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
            b.spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "".to_owned(),
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
            });
        });

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

fn wait_on_load(
    mut commands: Commands,
    foxes: Res<OneHundredThings>,
    barrier: Option<Res<AssetBarrier>>,
    loading: Query<Entity, With<Loading>>,
    gltfs: Res<Assets<Gltf>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if barrier.map(|b| b.is_ready()) != Some(true) {
        return;
    };
    commands.entity(loading.single()).despawn();
    commands.remove_resource::<AssetBarrier>();
    // Plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50000.0, 50000.0)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            ..default()
        },
        Loading,
    ));
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

fn get_async_loading_state(state: Res<AsyncLoadingState>, mut text: Query<&mut Text>) {
    state
        .0
        .lock()
        .unwrap()
        .clone_into(&mut text.single_mut().sections[0].value);
}
