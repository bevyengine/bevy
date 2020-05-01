use bevy::prelude::*;

/// This is a guided introduction to Bevy's "Entity Component System" (ECS)
/// All Bevy app logic is built using the ECS pattern, so definitely pay attention!
///
/// Why ECS?
/// * Data oriented: Functionality is driven by data
/// * Clean Architecture: Loose coupling of functionality / prevents deeply nested inheritance
/// * High Performance: Massively parallel and cache friendly
///
/// ECS Definitions:
///
/// Component: just a normal Rust data type. generally scoped to a single piece of functionality
///     Examples: position, velocity, health, color, name
///
/// Entity: a collection of components with a unique id
///     Examples: Entity1 { Name("Alice"), Position(0, 0) }, Entity2 { Name("Bill"), Position(10, 5) }

/// Resource: a shared global piece of data
///     Examples: asset_storage, events, system state
///
/// System: runs logic on entities, components, and resources
///     Examples: move_system, damage_system
///
/// Now that you know a little bit about ECS, lets look at some Bevy code!

// Our Bevy app's entry point
fn main() {
    // Bevy apps are created using the builder pattern. Here add our
    App::build()
        // This plugin runs our app's "system schedule" exactly once. Most apps will run on a loop,
        // but we don't want to spam your console with a bunch of example text :)
        .add_plugin(ScheduleRunnerPlugin::run_once())
        // Resources can be added to our app like this
        .add_resource(A { value: 1 })
        // Resources that implement the Default or FromResources trait can be added like this:
        .init_resource::<B>()
        .init_resource::<State>()
        // Systems can be added to our app like this
        // the system() call converts normal rust functions into ECS systems
        .add_system(empty_system.system())
        // Startup systems run exactly once BEFORE all other systems. These are generally used for
        // app initialization code (adding entities and resources)
        .add_startup_system(startup_system)
        // Systems that need resources to be constructed can be added like this
        .init_system(complex_system)
        // Here we just the rest of the example systems
        .add_system(resource_system.system())
        .add_system(for_each_entity_system.system())
        .add_system(resources_and_components_system.system())
        .add_system(command_buffer_system.system())
        .add_system(thread_local_system)
        .add_system(closure_system())
        .add_system(stateful_system.system())
        .run();
}

// RESOURCES: "global" state accessible by systems

struct A {
    value: usize,
}

#[derive(Default)]
struct B {
    value: usize,
}

struct C;

// COMPONENTS: pieces of functionality we add to entities

struct X {
    value: usize,
}
struct Y {
    value: usize,
}

// SYSTEMS: logic that runs on entities, components, and resources

// This is the simplest system. It will run once each time our app updates:
fn empty_system() {
    println!("hello!");
}

// Systems can also read and modify resources:
fn resource_system(a: Resource<A>, mut b: ResourceMut<B>) {
    b.value += 1;
    println!("resource_system: {} {}", a.value, b.value);
}

// This system runs once for each entity with the X and Y component
// NOTE: x is a read-only reference (Ref) whereas y can be modified (RefMut)
fn for_each_entity_system(x: Ref<X>, mut y: RefMut<Y>) {
    y.value += 1;
    println!("for_each_entity_system: {} {}", x.value, y.value);
}

// This system is the same as the above example, but it also accesses resource A
// NOTE: resources must always come before components in system functions
fn resources_and_components_system(a: Resource<A>, x: Ref<X>, mut y: RefMut<Y>) {
    y.value += 1;
    println!("resources_and_components:");
    println!("    components: {} {}", x.value, y.value);
    println!("    resource: {} ", a.value);
}

// This is a "startup" system that runs once when the app starts up. The only thing that distinguishes a
// startup" system from a "normal" system is how it is registered:
//      app.add_startup_system(startup_system)
//      app.add_system(normal_system)
// With startup systems we can create resources and add entities to our world, which can then be used by
// our other systems:
fn startup_system(world: &mut World, resources: &mut Resources) {
    // We already added A and B when we built our App above, so we don't re-add them here
    resources.insert(C);

    // Add some entities to our world
    world.insert(
        (),
        vec![
            (X { value: 0 }, Y { value: 1 }),
            (X { value: 2 }, Y { value: 3 }),
        ],
    );

    // Add some entities to our world
    world.insert(
        (),
        vec![
            (X { value: 0 }, Y { value: 1 }),
            (X { value: 2 }, Y { value: 3 }),
        ],
    );
}

// This system uses a command buffer to create a new entity on each iteration
// Normal systems cannot safely access the World instance because they run in parallel
// Command buffers give us the ability to queue up changes to our World without directly accessing it
// NOTE: Command buffers must always come before resources and components in system functions
fn command_buffer_system(command_buffer: &mut CommandBuffer, a: Resource<A>) {
    // Creates a new entity with a value read from resource A
    command_buffer.insert((), vec![(X { value: a.value },)]);
}

// If you really need full/immediate read/write access to the world or resources, you can use a "thread local system".
// These run on the main app thread (hence the name "thread local")
// WARNING: These will block all parallel execution of other systems until they finish, so they should generally be avoided
// NOTE: You may notice that this looks exactly like the "setup" system above. Thats because they are both thread local!
fn thread_local_system(world: &mut World, _resources: &mut Resources) {
    world.insert((), vec![(X { value: 1 },)]);
}

// These are like normal systems, but they also "capture" variables, which they can use as local state.
// This system captures the "counter" variable and uses it to maintain a count across executions
// NOTE: This function returns a Box<dyn Schedulable> type. If you are new to rust don't worry! All you
// need to know for now is that the Box contains our system AND the state it captured.
// You may recognize the .system() call from when we added our system functions to our App in the main()
// function above. Now you know that we are actually converting our functions into the Box<dyn Schedulable> type!
fn closure_system() -> Box<dyn Schedulable> {
    let mut counter = 0;
    (move |x: Ref<X>, mut y: RefMut<Y>| {
        y.value += 1;
        println!("closure_system: {} {}", x.value, y.value);
        println!("    ran {} times: ", counter);
        counter += 1;
    })
    .system()
}

// Closure systems should be avoided in general because they hide state from the ECS. This makes scenarios
// like "saving", "networking/multiplayer", and "replays" much harder.
// Instead you should use the "state" pattern whenever possible:

#[derive(Default)]
struct State {
    counter: usize,
}

fn stateful_system(mut state: RefMut<State>, x: Ref<X>, mut y: RefMut<Y>) {
    y.value += 1;
    println!("stateful_system: {} {}", x.value, y.value);
    println!("    ran {} times: ", state.counter);
    state.counter += 1;
}

// If you need more flexibility, you can define complex systems using "system builders".
// SystemBuilder enables scenarios like "multiple queries" and "query filters"
fn complex_system(_resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut counter = 0;
    SystemBuilder::new("complex_system")
        .read_resource::<A>()
        .write_resource::<B>()
        // this query is equivalent to the system we saw above: system(x: Ref<X>, y: RefMut<Y>)
        .with_query(<(Read<X>, Write<Y>)>::query())
        // this query only runs on entities with an X component that has changed since the last update
        .with_query(<Read<X>>::query().filter(changed::<X>()))
        .build(
            move |_command_buffer, world, (a, ref mut b), (x_y_query, x_changed_query)| {
                println!("complex_system:");
                println!("    resources: {} {}", a.value, b.value);
                for (x, mut y) in x_y_query.iter_mut(world) {
                    y.value += 1;
                    println!(
                        "    processed entity {} times: {} {}",
                        counter, x.value, y.value
                    );
                    counter += 1;
                }

                for x in x_changed_query.iter(world) {
                    println!("    x changed: {}", x.value);
                }
            },
        )
}
