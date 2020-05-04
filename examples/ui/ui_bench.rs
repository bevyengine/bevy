use bevy::prelude::*;
use bevy_ui::Rect;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .add_system(move_system.system())
        .add_plugin(DiagnosticsPlugin {
            print_diagnostics: true,
            ..Default::default()
        })
        .run();
}

fn move_system(time: Resource<Time>, mut node: RefMut<Node>, rect: Ref<Rect>) {
    if rect.color.r > 0.2 {
        node.position += Vec2::new(0.1 * time.delta_seconds, 0.0);
    }
}

fn setup(world: &mut World, _resources: &mut Resources) {
    let mut builder = world.build();
    builder.add_entity(Camera2dEntity {
        camera: Camera::new(CameraType::default_orthographic()),
        ..Default::default()
    });

    let mut prev = Vec2::default();
    let count = 1000;
    for i in 0..count {
        // 2d camera
        let cur = Vec2::new(1.0, 1.0) * 1.0 + prev;
        builder.add_entity(UiEntity {
            node: Node::new(
                math::vec2(75.0, 75.0) + cur,
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            rect: Rect {
                color: Color::rgb(0.0 + i as f32 / count as f32, 0.1, 0.1),
                ..Default::default()
            },
            ..Default::default()
        });

        prev = cur;
    }
}
