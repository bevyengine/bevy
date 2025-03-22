//! Desktop request redraw
use bevy::{prelude::*, window::RequestRedraw, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, (update, redraw.after(update)))
        .run();
}

#[derive(Component)]
struct AnimationActive;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            intensity: 1e6,
            ..Default::default()
        },
        Transform::from_xyz(-1.0, 5.0, 1.0),
    ));

    let node = Node {
        display: Display::Block,
        padding: UiRect::all(Val::Px(10.0)),
        row_gap: Val::Px(10.0),
        ..Default::default()
    };

    commands.spawn((
        node.clone(),
        children![
            (
                node.clone(),
                children![Text::new("Right click cube to pause animation")]
            ),
            (
                node.clone(),
                children![Text::new("Left click cube to start animation")]
            )
        ],
    ));

    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_length(1.0))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            AnimationActive,
        ))
        .observe(
            |trigger: Trigger<Pointer<Click>>, mut commands: Commands| match trigger.button {
                PointerButton::Primary => {
                    commands.entity(trigger.target()).insert(AnimationActive);
                }
                PointerButton::Secondary => {
                    commands
                        .entity(trigger.target())
                        .remove::<AnimationActive>();
                }
                _ => {}
            },
        );
}

fn update(time: Res<Time>, mut query: Query<&mut Transform, With<AnimationActive>>) {
    if let Ok(mut transform) = query.single_mut() {
        transform.rotate_x(time.delta_secs().min(1.0 / 60.0));
    }
}

fn redraw(mut commands: Commands, query: Query<Entity, With<AnimationActive>>) {
    if query.iter().next().is_some() {
        commands.send_event(RequestRedraw);
    }
}
