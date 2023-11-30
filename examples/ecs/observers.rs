//! This examples illustrates the different ways you can employ observers

use bevy::prelude::*;

#[derive(Component, Debug)]
struct MyComponent(usize);

#[derive(Component)]
struct MyEvent(usize);

#[derive(Resource, Default)]
struct MyResource(usize);

fn main() {
    App::new().add_systems(Startup, setup).run();
}

fn setup(world: &mut World) {
    world.init_resource::<MyResource>();

    // Responds to all added instances of MyComponent (or any WorldQuery/Filter)
    world.observer(|mut observer: Observer<OnAdd, &MyComponent>| {
        let mut resource = observer.world_mut().resource_mut::<MyResource>();
        resource.0 += 1;

        let count = resource.0;
        let my_component = observer.fetch().0;
        println!(
            "Added: {:?} to entity: {:?}, count: {:?}",
            my_component,
            observer.source(),
            count
        );
    });

    let entity_a = world.spawn(MyComponent(0)).flush();

    // Responds to MyEvent events targeting this entity
    let entity_b = world
        .spawn(MyComponent(1))
        .observe(|mut observer: Observer<MyEvent, &mut MyComponent>| {
            let data = observer.data().0;
            let mut my_component = observer.fetch();
            my_component.0 += 1;
            println!("Component: {:?}, Event: {:?}", my_component.0, data);
        })
        .flush();

    world.ecs_event(MyEvent(5)).target(entity_b).emit();
    world.ecs_event(MyEvent(10)).target(entity_a).emit();
    world.ecs_event(MyEvent(15)).target(entity_b).emit();
}
