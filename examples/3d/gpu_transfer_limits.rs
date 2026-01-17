//! This examples demonstrates the usage of RenderAssetBytesPerFrame and RenderAssetTransferPriority
//! for managing gpu transfer rates and avoiding frame hiccups
use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_asset::{RenderAssetTransferPriority, RenderAssetUsages};
use bevy_render::{
    render_asset::{
        RenderAssetBytesPerFrame, RenderAssetBytesPerFrameLimiter,
        RenderAssetPriorityAllocationStats,
    },
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    Extract, RenderApp,
};

fn main() {
    let mut app = App::new();

    let (sender, receiver) = crossbeam_channel::unbounded();

    // note: 1kb is a VERY low limit, only useful for demonstrating the functionality visually.
    // low-end hardware will not see any benefit at 60fps below ~50mb (~3gb / sec)
    app.insert_resource(RenderAssetBytesPerFrame::MaxBytesWithPriority(1000))
        .insert_resource(StatsChannel { sender, receiver })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update, update_stats));

    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_systems(ExtractSchedule, extract_stats);
    app.run();
}

#[derive(Component)]
struct PlaneColor([u8; 3], RenderAssetTransferPriority);

#[derive(Component)]
struct UiOutput;

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // plane mesh
    let mut mesh = Plane3d::new(Vec3::Z, Vec2::splat(0.5)).mesh().build();
    mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
    mesh.transfer_priority = RenderAssetTransferPriority::Immediate;
    let plane_mesh = meshes.add(mesh);

    // set up priorities
    let mut priorities: [RenderAssetTransferPriority; 6] = Default::default();
    priorities[0] = RenderAssetTransferPriority::Immediate;
    for i in 1..6 {
        priorities[i] = RenderAssetTransferPriority::Priority(10 - i as i16);
    }

    // spawn planes showing images with varying priorities
    for (y, priority) in priorities.iter().enumerate() {
        for x in 0..100 {
            let color = [x as u8 * 2, 128, y as u8 * 50];
            let image = images.add(Image::new_fill(
                Extent3d {
                    width: 25,
                    height: 10,
                    depth_or_array_layers: 1, // 1000 bytes per image
                },
                TextureDimension::D2,
                &[color[0], color[1], color[2], 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
                *priority,
            ));
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(image),
                unlit: true,
                ..Default::default()
            });
            commands.spawn((
                Transform::from_translation(Vec3::new(x as f32 - 49.5, 2.0 - y as f32, 0.0)),
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(material),
                PlaneColor(color, *priority),
            ));
        }
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::Z * 75.0),
    ));

    // stats
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(5)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.75)),
        GlobalZIndex(i32::MAX),
        children![(
            (Text::default(), UiOutput),
            children![
                (TextSpan::new("GPU Transfer Limits:\n")),
                (TextSpan::new("Press Space to modify all materials\n\n")),
                (TextSpan::new(format!(
                    "{:^20}{:^20}{:^15}{:^20}{:^15}{:^15}\n",
                    "Priority",
                    "requested bytes",
                    "requested count",
                    "written bytes",
                    "written count",
                    "allocated bytes"
                )))
            ]
        )],
    ));
}

fn update(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planes: Query<(&MeshMaterial3d<StandardMaterial>, &PlaneColor)>,
    mut toggled: Local<bool>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        *toggled = !*toggled;
        for (material_handle, spec) in planes.iter() {
            let color = if *toggled {
                [255 - spec.0[0], 255 - spec.0[1], 255 - spec.0[2], 255]
            } else {
                [spec.0[0], spec.0[1], spec.0[2], 255]
            };
            // create a new image
            let image = images.add(Image::new_fill(
                Extent3d {
                    width: 25,
                    height: 10,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &color,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
                spec.1,
            ));

            // note: we must modify the existing material if we want to retain the old version
            // until the new version is transferred.
            // if we created a new material, the old one would be immediately removed as unused, and nothing
            // would be displayed until the new image is transferred.
            let material = materials.get_mut(&material_handle.0).unwrap();
            material.base_color_texture = Some(image);
        }
    }
}

#[derive(Resource)]
struct StatsChannel {
    sender: crossbeam_channel::Sender<
        BTreeMap<RenderAssetTransferPriority, RenderAssetPriorityAllocationStats>,
    >,
    receiver: crossbeam_channel::Receiver<
        BTreeMap<RenderAssetTransferPriority, RenderAssetPriorityAllocationStats>,
    >,
}

fn extract_stats(
    channel: Extract<Res<StatsChannel>>,
    limiter: Res<RenderAssetBytesPerFrameLimiter>,
) {
    let stats = limiter.stats();
    let _ = channel.sender.send(stats);
}

fn update_stats(
    mut commands: Commands,
    output: Query<(Entity, &Children), With<UiOutput>>,
    stats: Res<StatsChannel>,
) {
    if let Ok(stats) = stats.receiver.try_recv() {
        let (output_entity, output_children) = output.single().unwrap();
        for (i, (priority, stat)) in stats.iter().rev().enumerate() {
            let text = TextSpan::new(format!(
                "{:^20}{:^20}{:^15}{:^20}{:^15}{:^15}\n",
                match priority {
                    RenderAssetTransferPriority::Immediate => format!("Immediate"),
                    RenderAssetTransferPriority::Priority(p) => format!("Priority {p}"),
                },
                stat.requested_bytes,
                stat.requested_count,
                stat.written_bytes,
                stat.written_count,
                stat.available_bytes,
            ));
            if let Some(child) = output_children.get(i + 3) {
                commands.entity(*child).insert(text);
            } else {
                commands.spawn((text, ChildOf(output_entity)));
            }
        }
        println!("{}", output_children.len());
    } else {
        error!("no stats");
    }
}
