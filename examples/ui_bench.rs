use bevy::prelude::*;

fn main() {
    App::build()
        .add_defaults()
        .setup(setup)
        .add_system(build_move_system())
        .add_plugin(DiagnosticsPlugin {
            print_diagnostics: true,
            ..Default::default()
        })
        .run();
}

fn build_move_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("move")
        .read_resource::<Time>()
        .with_query(<Write<Node>>::query())
        .build(move |_, world, time, query| {
            for (_i, mut node) in query.iter_mut(world).enumerate() {
                if node.color.r > 0.2 {
                    node.position += Vec2::new(0.1 * time.delta_seconds, 0.0);
                    // println!("{}", node.position.x());
                }
            }
        })
}

fn setup(world: &mut World, _resources: &mut Resources) {
    let mut builder = world.build().add_entity(Camera2dEntity {
        camera: Camera::new(CameraType::default_orthographic()),
        ..Default::default()
    });

    let mut prev = Vec2::default();
    let count = 1000;
    for i in 0..count {
        // 2d camera
        let cur = Vec2::new(1.0, 1.0) * 1.0 + prev;
        builder = builder.add_entity(UiEntity {
            node: Node::new(
                math::vec2(75.0, 75.0) + cur,
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgb(0.0 + i as f32 / count as f32, 0.1, 0.1),
            ),
        });

        prev = cur;
    }

    builder.build();
}
