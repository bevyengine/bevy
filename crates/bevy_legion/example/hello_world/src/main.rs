use legion::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vel(f32, f32, f32);

#[derive(Clone)]
pub struct ExampleResource1(String);
#[derive(Clone)]
pub struct ExampleResource2(String);

fn main() {
    let _ = tracing_subscriber::fmt::try_init();

    // create world
    let universe = Universe::new();
    let mut world = universe.create_world();

    // Create resources
    // Resources are also dynamically scheduled just like components, so the accesses
    // declared within a SystemBuilder are correct.
    // Any resource accessed by systems *must be* manually inserted beforehand, otherwise it will panic.
    let mut resources = Resources::default();
    resources.insert(ExampleResource1("ExampleResource1".to_string()));
    resources.insert(ExampleResource2("ExampleResource2".to_string()));

    // create entities
    // An insert call is used to insert matching entities into the world.
    let entities = world
        .insert(
            (),
            vec![
                (Pos(1., 2., 3.), Vel(1., 2., 3.)),
                (Pos(1., 2., 3.), Vel(1., 2., 3.)),
                (Pos(1., 2., 3.), Vel(1., 2., 3.)),
                (Pos(1., 2., 3.), Vel(1., 2., 3.)),
            ],
        )
        .to_vec();

    // update positions
    // This example shows the use of a `iter`, which is default mutable, across a query.
    let query = <(Write<Pos>, Read<Vel>)>::query();
    for (mut pos, vel) in query.iter_mut(&mut world) {
        pos.0 += vel.0;
        pos.1 += vel.1;
        pos.2 += vel.2;
    }

    // update positions using a system
    let update_positions = SystemBuilder::new("update_positions")
        .write_resource::<ExampleResource1>()
        .read_resource::<ExampleResource2>()
        .with_query(<(Write<Pos>, Read<Vel>)>::query())
        .build(|_, world, (res1, res2), query| {
            res1.0 = res2.0.clone(); // Write the mutable resource from the immutable resource

            for (mut pos, vel) in query.iter_mut(world) {
                pos.0 += vel.0;
                pos.1 += vel.1;
                pos.2 += vel.2;
            }
        });

    // Uses the command buffer to insert an entity into the world every frame.
    let entity = entities[0];
    let command_buffer_usage = SystemBuilder::new("command_buffer_usage")
        .read_resource::<ExampleResource1>()
        .write_resource::<ExampleResource2>()
        // Read and write component definitions allow us to declare access to a component across all archetypes
        // This means we can use the SubWorld provided to the system as a `World` for that component.
        .write_component::<Pos>()
        .build(move |command_buffer, world, (res1, res2), _| {
            res2.0 = res1.0.clone(); // Write the mutable resource from the immutable resource

            // Read a component from the SubWorld.
            let _ = world.get_component_mut::<Pos>(entity).unwrap();

            let _entities = command_buffer.insert(
                (),
                vec![
                    (Pos(1., 2., 3.), Vel(1., 2., 3.)),
                    (Pos(1., 2., 3.), Vel(1., 2., 3.)),
                ],
            );
        });

    let thread_local_example = Box::new(|world: &mut World, _resources: &mut Resources| {
        // This is an example of a thread local system which has full, exclusive mutable access to the world.
        let query = <(Write<Pos>, Read<Vel>)>::query();
        for (mut pos, vel) in query.iter_mut(world) {
            pos.0 += vel.0;
            pos.1 += vel.1;
            pos.2 += vel.2;
        }
    });

    let mut schedule = Schedule::builder()
        .add_system(update_positions)
        .add_system(command_buffer_usage)
        // This flushes all command buffers of all systems.
        .flush()
        // a thread local system or function will wait for all previous systems to finish running,
        // and then take exclusive access of the world.
        .add_thread_local_fn(thread_local_example)
        .build();

    // Execute a frame of the schedule.
    schedule.execute(&mut world, &mut resources);
}
