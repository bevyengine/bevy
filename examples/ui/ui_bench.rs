use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(placement_system.system())
        .run();
}

fn placement_system(
    time: Res<Time>,
    materials: Res<Assets<ColorMaterial>>,
    mut query: Query<(&mut Style, &Handle<ColorMaterial>)>,
) {
    for (mut style, material_handle) in &mut query.iter() {
        let material = materials.get(&material_handle).unwrap();
        if material.color.r > 0.2 {
            style.position.left += 0.1 * time.delta_seconds;
        }
    }
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    // 2d camera
    commands.spawn(Camera2dComponents::default());

    let mut prev = Vec2::default();
    let count = 1000;
    for i in 0..count {
        let cur = Vec2::new(1.0, 1.0) + prev;
        commands.spawn(NodeComponents {
            style: Style {
                size: Size {
                    width: Val::Px(100.0),
                    height: Val::Px(100.0),
                },
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(75.0 + cur.x()),
                    bottom: Val::Px(75.0 + cur.y()),
                    ..Default::default()
                },
                ..Default::default()
            },
            material: materials.add(Color::rgb(0.0 + i as f32 / count as f32, 0.1, 0.1).into()),
            ..Default::default()
        });

        prev = cur;
    }
}
