use bevy::prelude::*;

/// Illustrates the different ways you can declare systems
fn main() {
    App::build()
        .add_default_plugins()
        .add_event::<MyEvent>()
        .add_startup_system(setup)
        .add_system_init(built_system)
        .add_system(simple_system.into_system("simple_system"))
        .add_system(closure_system())
        .run();
}

struct MyEvent(usize);

// resources
struct A(usize);

// components
struct X(usize);
struct Y(usize);

// add our resources and entities
fn setup(world: &mut World, resources: &mut Resources) {
    resources.insert(A(0));
    world.insert((), vec![(X(0), Y(1)), (X(2), Y(3))]);
}

// runs once for each entity with the X and Y component
fn simple_system(x: Ref<X>, mut y: RefMut<Y>) {
    y.0 += 1;
    println!("processed entity: {} {}", x.0, y.0);
}

// does the same thing as the first system, but also captures the "counter" variable and uses it as internal state
fn closure_system() -> Box<dyn Schedulable> {
    let mut counter = 0;
    (move |x: Ref<X>, mut y: RefMut<Y>| {
        y.0 += 1;
        println!("processed entity: {} {}", x.0, y.0);
        println!("ran {} times", counter);
        counter += 1;
    })
    .into_system("closure_system")
}

// if you need more flexibility, you can define complex systems using the system builder
fn built_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut my_event_reader = resources.get_event_reader::<MyEvent>();
    SystemBuilder::new("example")
        .read_resource::<Events<MyEvent>>()
        .write_resource::<A>()
        .with_query(<(Read<X>, Write<Y>)>::query())
        .build(
            move |_command_buffer, world, (my_events, ref mut a), query| {
                for event in my_event_reader.iter(&my_events) {
                    a.0 += event.0;
                    println!("modified resource A with event: {}", event.0);
                }
                for (x, mut y) in query.iter_mut(world) {
                    y.0 += 1;
                    println!("processed entity: {} {}", x.0, y.0);
                }
            },
        )
}
