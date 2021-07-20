use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::camera::Camera,
    sprite::SpriteSettings,
};
use rand::Rng;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // Adds frame time diagnostics
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // Adds a system that prints diagnostics to the console
        .add_plugin(LogDiagnosticsPlugin::default())
        .insert_resource(GroupSet(Vec::new()))
        .insert_resource(SpriteSettings {
            // NOTE: this is an experimental feature that doesn't work in all cases
            frustum_culling_enabled: true,
        })
        .add_startup_system(create_groups.system())
        .add_system(move_camera.system())
        //
        .add_system_to_stage(CoreStage::PreUpdate, remove_group_targets.system())
        //
        .add_stage_after(CoreStage::PreUpdate, "foo", SystemStage::parallel())
        .add_system_to_stage("foo", pick_group_targets.system())
        //
        .add_system(move_bevys.system().label("a"))
        .add_system(set_group_position.system().after("a"))
        .run()
}

struct InGroup;
struct MoveToGroup;

struct Group;

struct TargetOffset(Vec3);
struct GroupPosition(Vec3);

struct GroupSet(Vec<Entity>);

const NUM_GROUPS: u32 = 30;
const MAP_BOUNDS: (f32, f32) = (1500., 750.);
const GROUP_RANGE: (f32, f32) = (100., 100.);

fn create_groups(
    mut commands: Commands,
    mut groups: ResMut<GroupSet>,
    assets: ResMut<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::thread_rng();

    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d());

    for _ in 0..NUM_GROUPS {
        let sprite_handle = materials.add(assets.load("branding/icon.png").into());
        materials.get_mut(&sprite_handle).unwrap().color =
            Color::rgb(rng.gen(), rng.gen(), rng.gen());

        let group_area = Vec3::new(
            rng.gen_range(-MAP_BOUNDS.0..MAP_BOUNDS.0),
            rng.gen_range(-MAP_BOUNDS.1..MAP_BOUNDS.1),
            0.,
        );

        let group_id = commands
            .spawn()
            .insert(Group)
            .insert(GroupPosition(group_area))
            .id();
        groups.0.push(group_id);

        for _ in 0..rng.gen_range(50..150) {
            commands
                .spawn()
                .insert_relation(InGroup, group_id)
                .insert(TargetOffset(Vec3::new(
                    rng.gen_range(-GROUP_RANGE.0..GROUP_RANGE.0),
                    rng.gen_range(-GROUP_RANGE.1..GROUP_RANGE.1),
                    0.,
                )))
                .insert_bundle(SpriteBundle {
                    material: sprite_handle.clone(),
                    transform: Transform::from_translation(
                        Vec3::new(
                            rng.gen_range(-GROUP_RANGE.0..GROUP_RANGE.0),
                            rng.gen_range(-GROUP_RANGE.1..GROUP_RANGE.1),
                            0.,
                        ) + group_area,
                    ),
                    sprite: Sprite::new(Vec2::new(48., 48.)),
                    ..Default::default()
                });
        }
    }
}

fn move_camera(keys: Res<Input<KeyCode>>, mut query: Query<&mut Transform, With<Camera>>) {
    let mut transform = query.single_mut().unwrap();
    if keys.pressed(KeyCode::A) {
        transform.translation -= Vec3::new(10., 0., 0.);
    }
    if keys.pressed(KeyCode::D) {
        transform.translation += Vec3::new(10., 0., 0.);
    }
    if keys.pressed(KeyCode::W) {
        transform.translation += Vec3::new(0., 10., 0.);
    }
    if keys.pressed(KeyCode::S) {
        transform.translation -= Vec3::new(0., 10., 0.);
    }
    transform.translation.x = transform.translation.x.clamp(-MAP_BOUNDS.0, MAP_BOUNDS.0);
    transform.translation.y = transform.translation.y.clamp(-MAP_BOUNDS.1, MAP_BOUNDS.1);
}

fn set_group_position(
    mut bevys: Query<&Transform, With<Relation<InGroup>>>,
    mut groups: Query<&mut GroupPosition>,
    group_set: Res<GroupSet>,
) {
    for &group_entity in group_set.0.iter() {
        let mut iter = bevys
            .new_target_filters(TargetFilter::<InGroup>::new().target(group_entity))
            .apply_filters()
            .iter();

        let mut average_pos =
            (iter.next().unwrap().translation + iter.next().unwrap().translation) / 2.0;

        for pos in iter {
            average_pos += pos.translation;
            average_pos /= 2.0;
        }

        let mut group_pos = groups.get_mut(group_entity).unwrap();
        group_pos.0 = average_pos;
    }
}

fn remove_group_targets(
    mut commands: Commands,
    mut groups: Query<(&mut GroupPosition, &Relation<MoveToGroup>), With<Group>>,
    group_set: Res<GroupSet>,
) {
    for &group_entity in group_set.0.iter() {
        let (group_pos, move_to) = match groups.get_mut(group_entity) {
            Ok(mut components) => ((*components.0).0, components.1.single().0),
            Err(_) => continue,
        };
        let target_pos = match groups.get_mut(move_to) {
            Ok((target_pos, _)) => (*target_pos).0,
            Err(_) => continue,
        };

        if (target_pos - group_pos).abs().length() < 100.0 {
            commands
                .entity(group_entity)
                .remove_relation::<MoveToGroup>(move_to);
        }
    }
}

fn pick_group_targets(
    mut commands: Commands,
    groups: Query<Entity, (With<Group>, Without<Relation<MoveToGroup>>)>,
    group_set: Res<GroupSet>,
) {
    let mut rng = rand::thread_rng();
    for group in groups.iter() {
        let group_target = loop {
            let group_target = group_set.0[rng.gen_range::<usize, _>(0..group_set.0.len())];
            if group_target != group {
                break group_target;
            }
        };

        commands
            .entity(group)
            .insert_relation(MoveToGroup, group_target);
    }
}

fn move_bevys(
    mut bevys: Query<(&mut Transform, &TargetOffset, &Relation<InGroup>)>,
    mut groups: Query<(&mut GroupPosition, &Relation<MoveToGroup>)>,
    group_set: Res<GroupSet>,
) {
    for &group_entity in group_set.0.iter() {
        let (move_to, _) = match groups.get_mut(group_entity) {
            Ok(mut x) => x.1.single(),
            Err(_) => continue,
        };
        let GroupPosition(target_pos) = *groups.get_mut(move_to).unwrap().0;

        bevys
            .new_target_filters(TargetFilter::<InGroup>::new().target(group_entity))
            .apply_filters()
            .iter_mut()
            .for_each(|(mut transform, target_offset, _)| {
                let velocity = ((target_pos + target_offset.0) - transform.translation).normalize();
                transform.translation += velocity;
            });
    }
}
