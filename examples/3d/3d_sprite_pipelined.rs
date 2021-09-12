use bevy::{
    asset::LoadState,
    ecs::query::With,
    math::Quat,
    prelude::{
        App, AssetServer, Commands, HandleUntyped, Query, Res, ResMut, State, SystemSet, Time,
        Transform,
    },
    render2::{camera::PerspectiveCameraBundle, texture::Image},
    PipelinedDefaultPlugins,
};

use bevy::prelude::*;
use bevy::sprite2::Sprite3dBundle;

fn main() {
    App::new()
        .init_resource::<TexHandle>()
        .add_plugins(PipelinedDefaultPlugins)
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(load_textures))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_textures))
        .add_system_set(SystemSet::on_enter(AppState::Finished).with_system(setup))
        .add_system_set(SystemSet::on_update(AppState::Finished).with_system(rotation_system))
        .add_system_set(SystemSet::on_update(AppState::Finished).with_system(zoom_system))
        .run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Finished,
}

#[derive(Default)]
struct TexHandle {
    handle: Option<HandleUntyped>,
}

fn load_textures(mut tex_handle: ResMut<TexHandle>, asset_server: Res<AssetServer>) {
    tex_handle.handle = Some(
        asset_server
            .load::<Image, _>("branding/icon.png")
            .clone_untyped(),
    );
}

fn check_textures(
    mut state: ResMut<State<AppState>>,
    tex_handle: ResMut<TexHandle>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_load_state(tex_handle.handle.as_ref().unwrap().id) {
        state.set(AppState::Finished).unwrap();
    }
}

fn setup(mut commands: Commands, tex_handle: Res<TexHandle>) {
    let entity = commands
        .spawn_bundle(Sprite3dBundle {
            texture: tex_handle.handle.as_ref().unwrap().clone().typed(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .id();

    let mut camera = PerspectiveCameraBundle::new_3d();
    camera.transform = Transform {
        translation: Vec3::new(0.0, 0.0, 1000.0),
        ..Default::default()
    };

    commands.spawn_bundle(camera).insert(Rotate).insert(Zoom {
        target: entity,
        zooming: Zooming::In,
    });
}

struct Rotate;

fn rotation_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotate>>) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(Quat::from_rotation_z(
            (4.0 * std::f32::consts::PI / 30.0) * time.delta_seconds(),
        )) * Transform::from_rotation(Quat::from_rotation_y(
            (4.0 * std::f32::consts::PI / 20.0) * time.delta_seconds(),
        )) * Transform::from_rotation(Quat::from_rotation_x(
            (4.0 * std::f32::consts::PI / 16.0) * time.delta_seconds(),
        )) * *transform;
    }
}

struct Zoom {
    target: Entity,
    zooming: Zooming,
}

enum Zooming {
    In,
    Out,
}

fn zoom_system(
    time: Res<Time>,
    mut queries: QuerySet<(
        Query<(&mut Transform, &mut Zoom)>,
        Query<&Transform, Without<Zoom>>,
    )>,
) {
    for (mut camera_transform, mut zoom) in queries.q0_mut().iter_mut() {
        let target_transform = queries.q1().get(zoom.target).unwrap();

        let diff = camera_transform.translation - target_transform.translation;
        let distance = diff.length();
        let dir = diff.normalize();

        match zoom.zooming {
            Zooming::In => {
                if distance < 100.0 {
                    zoom.zooming = Zooming::Out;
                }
                camera_transform.translation = dir * (distance - 1000.0 * time.delta_seconds());
            }
            Zooming::Out => {
                if distance > 5000.0 {
                    zoom.zooming = Zooming::In;
                }
                camera_transform.translation = dir * (distance + 1000.0 * time.delta_seconds());
            }
        }
    }
}
