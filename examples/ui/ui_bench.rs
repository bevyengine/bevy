use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .add_system(placement_system.system())
        .add_plugin(DiagnosticsPlugin::default())
        .run();
}

fn placement_system(
    time: Resource<Time>,
    materials: Resource<Assets<ColorMaterial>>,
    mut node: RefMut<Node>,
    material_handle: Ref<Handle<ColorMaterial>>,
) {
    let material = materials.get(&material_handle).unwrap();
    if material.color.r > 0.2 {
        node.position += Vec2::new(0.1 * time.delta_seconds, 0.0);
    }
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut builder = world.build();
    builder.add_entity(Camera2dEntity::default());

    let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
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
            material: materials.add(Color::rgb(0.0 + i as f32 / count as f32, 0.1, 0.1).into()),
            ..Default::default()
        });

        prev = cur;
    }
}
