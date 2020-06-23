# Legion Systems

This crate provides systems and a scheduler for the [legion](https://crates.io/crates/legion) ECS library.

Systems represent a unit of program logic which creates, deletes and manipulates the entity in a `World`. Systems can declare queries which define how they access entity data, and can declare access to shared `resources`. Systems can be compiled into a `Schedule`, which is executed each frame.

# Defining Systems

Systems are constructed via the `SystemBuilder`. The function body of the system is given as a closure.

By default, a system is provided with a command buffer and a sub-world. The sub-world allows access to entity data, but entities cannot be immediately created or deleted. Instead, entity creation command can be queued into the command buffer. Command buffers are flushed at the end of the schedule by default.

```
let system = SystemBuilder::new("my system")
    .build(|command_buffer, world, _, _| {
        // create a new entity
        command_buffer.insert(Tag, vec![(Pos, Vel)]);
    });
```

Queries can be added to a system to allow the system to access entity data, and shared data can be accessed via resources:

```
let system = SystemBuilder::new("my system")
    .with_query(<(Read<Vel>, Write<Pos>)>::query())
    .reads_resource::<Time>()
    .build(|command_buffer, world, time, query| {
        for (vel, pos) in query.iter_mut(world) {
            *pos += *vel * time;
        }
    });
```

Multiple queries can resources can be requested. The third and fouth closure parameters will contain a tuple of all resources and queries, respectively.

Systems can be compiled into a `Schedule`. And the schedule executed. Schedules will automatically parallelize systems where possible, based upon the resource and entity accesses declared by each system. Side effects (such as writing to a component) will be visible in the order in which systems were given to the schedule. System command buffers are flushed at the end of the schedule by default.

```
// create shared resources
let mut resources = Resources::default();
resources.insert(Time::new());

// define our system schedule
let mut schedule = Schedule::builder()
    .add_system(update_positions)
    .add_system(handle_collisions)
    .flush() // flush command buffers so later systems can see new entities
    .add_system(render)
    .build();

// each frame, execute the schedule
schedule.execute(&mut world, &mut resources);
```