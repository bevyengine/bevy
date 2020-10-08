# hecs

[![Documentation](https://docs.rs/hecs/badge.svg)](https://docs.rs/hecs/)
[![Crates.io](https://img.shields.io/crates/v/hecs.svg)](https://crates.io/crates/hecs)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

hecs provides a high-performance, minimalist entity-component-system (ECS)
world. It is a library, not a framework. In place of an explicit "System"
abstraction, a `World`'s entities are easily queried from regular code. Organize
your application however you like!

### Bevy Fork Information

This is the Bevy project's fork of hecs with changes that accommodate the needs of the Bevy game engine. Some notable changes:
* Entity indices are now queryable and are not returned in queries by default. This both improves ergonomics and significantly boosts performance in some cases.
* Expose more interfaces as public so that we can build higher-level apis on top of the core hecs codebase (multithreading, functions-as-systems, world builders, schedules, etc)
* Change Tracking 

### Why ECS?

Entity-component-system architecture makes it easy to compose loosely-coupled
state and behavior. An ECS world consists of:

- any number of **entities**, which represent distinct objects
- a collection of **component** data associated with each entity, where each
  entity has at most one component of any type, and two entities may have
  different components

That world is then manipulated by **systems**, each of which accesses all
entities having a particular set of component types. Systems implement
self-contained behavior like physics (e.g. by accessing "position", "velocity",
and "collision" components) or rendering (e.g. by accessing "position" and
"sprite" components).

New components and systems can be added to a complex application without
interfering with existing logic, making the ECS paradigm well suited to
applications where many layers of overlapping behavior will be defined on the
same set of objects, particularly if new behaviors will be added in the
future. This flexibility sets it apart from traditional approaches based on
heterogeneous collections of explicitly defined object types, where implementing
new combinations of behaviors (e.g. a vehicle which is also a questgiver) can
require far-reaching changes.

#### Performance

In addition to having excellent composability, the ECS paradigm can also provide
exceptional speed and cache locality. `hecs` internally tracks groups of
entities which all have the same components. Each group has a dense, contiguous
array for each type of component. When a system accesses all entities with a
certain set of components, a fast linear traversal can be made through each
group having a superset of those components. This is effectively a columnar
database, and has the same benefits: the CPU can accurately predict memory
accesses, bypassing unneeded data, maximizing cache use and minimizing latency.

### Why Not ECS?

An ECS world is not a be-all end-all data structure. Most games will store
significant amounts of state in other structures. For example, many games
maintain a spatial index structure (e.g. a tile map or bounding volume
hierarchy) used to find entities and obstacles near a certain location for
efficient collision detection without searching the entire world.

If you need to search for specific entities using criteria other than the types
of their components, consider maintaining a specialized index beside your world,
storing `Entity` handles and whatever other data is necessary. Insert into the
index when spawning relevant entities, and include a component with that allows
efficiently removing them from the index when despawning.

### Other Libraries

hecs would not exist if not for the great work done by others to introduce and
develop the ECS paradigm in the Rust ecosystem. In particular:

- [specs] played a key role in popularizing ECS in Rust
- [legion] reduced boilerplate and improved cache locality with sparse
  components

hecs builds on these successes by focusing on further simplification, boiling
the paradigm down to a minimal, light-weight and ergonomic core, without
compromising on performance or flexibility.

### Disclaimer

This is not an official Google product (experimental or otherwise), it is just
code that happens to be owned by Google.

[specs]: https://github.com/amethyst/specs
[legion]: https://github.com/TomGillen/legion
