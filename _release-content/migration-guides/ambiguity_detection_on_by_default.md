---
title: Schedule ambiguity detection is now on by default.
pull_requests: [26047]
---

System ambiguities are when two systems have a conflicting access on a component/resource (i.e.,
both mutate the component, or one mutates and one reads), but do **not** have a system ordering
between them. When this happens, the systems are run in a **random** order (and strictly speaking,
one that may change from frame to frame). This is **usually** a mistake, and leads to surprising
behavior.

For a few Bevy versions, Bevy crates have had zero system ambiguities. As such, we've decided to
enable system ambiguity detection **by default**. Ambiguity detection itself is not a new feature,
but as this is a large change for users, we've included a more thorough explanation below.

## Resolving ambiguities

There are several ways to resolve a system ambiguity.

### Apply an ordering

The most obvious solution is to just specify an explicit ordering between two systems. For example,
if you have:

```rust
fn system_a(_: Query<&mut Transform>) {}
fn system_b(_: Query<&mut Transform>) {}

fn main() {
  App::new()
    .add_systems(Update, system_a)
    // Add an explicit ordering to resolve the ambiguity!
    .add_systems(Update, system_b.after(system_a))
    .run();
}
```

Ideally, you should do this if the systems actually conflict, which may not be the case (see below).

Importantly, adding explicit ordering between systems can **reduce** parallelism in some cases,
since Bevy must ensure that the systems run in that particular order (whereas without an explicit
ordering, the systems can run as long as no conflicting system runs).

### Marking systems as ambiguous

Ambiguity detection can only track ambiguities of access to a component/resource. It **cannot**
determine whether the access you are doing actually "interacts". For example:

```rust
struct Thing {
  a: i32,
  b: i32,
}

fn add_to_a(things: Query<&mut Thing>) {
  for mut thing in things.iter_mut() {
    thing.a += 1;
  }
}
fn add_to_b(things: Query<&mut Thing>) {
  for mut thing in things.iter_mut() {
    thing.b += 1;
  }
}
```

Despite the fact that these systems both mutate the same component, they don't touch any of the same
data! So the order that these systems run doesn't matter! We can tell Bevy to ignore this ambiguity
like so:

```rust
app
  .add_systems(Update, add_to_a)
  .add_systems(Update, add_to_b.ambiguous_with(add_to_a));
```

Some other tools for marking ambiguities:

- `IntoScheduleConfigs::ambiguous_with`: This works with both systems **and** system sets.
- `IntoScheduleConfigs::ambiguous_with_all`: Marks that the added systems are ambiguous with **any** other system.
- `App::ignore_ambiguity`: Add an ambiguity **after** the systems have been added (useful for dealing with unruly crates).

Seriously consider whether you want to make systems ambiguous though. If your systems change to be
in conflict later (e.g., `add_to_b` starts reading `a` to change how it adds to `b`), ambiguity
detection will **not** notify you, since you "signed off" on this ambiguity. **Crate authors in
particular** should consider whether they provide the right tools for you to correctly describe your
ordering requirements and ignore system ambiguities as a last resort.

### Marking components/resources as ambiguous

Some components/resources may have interfaces that are *designed* to be ambiguous. For example,
consider this resource:

```rust
#[derive(Resource)]
pub struct MaterialCache(HashMap<u32, Handle<CustomMaterial>>);

impl MaterialCache {
  pub fn get_material_for_wobbliness(&mut self, wobbliness: u32, asset_server: &AssetServer) -> Handle<CustomMaterial> {
    match self.0.entry(wobbliness) {
      Entry::Occupied(entry) => entry.get().clone(),
      Entry::Vacant(entry) => {
        let handle = asset_server.add(make_wobbly_material(wobbliness));
        entry.insert(handle.clone());
        handle
      }
    }
  }
}
```

Despite the fact that the material cache requires a mutable borrow (in order to store the handles in
the cache), the order in which systems mutate this resource **does not matter**. In this case, it is
more convenient to mark the resource *itself* as ambiguous. This can be done easily with:

```rust
app.allow_ambiguous_resource::<MaterialCache>();
// You can also do the same for components!
app.allow_ambiguous_component::<SomeComponent>();
```

Again, ensure that your component or resource **really** should be marked ambiguous. Bevy will not
notify you if your API changes to where ordering does actually matter. And again, **crate authors**
should be extra sure, to prevent cases where a component or resource is marked ambiguous that then
becomes an unnoticed inconsistency for end users.

## Caveats with ambiguity detection

There are two remaining caveats with ambiguity detection:

### 1. Interior mutability cannot be detected

Bevy does not consider two systems reading the same component to be a conflict, and therefore not an
ambiguity. If your component/resource uses an interior mutability type like a `Mutex` (allowing you
to mutate the component/resource through a shared reference), it is up to you to prevent
ambiguities.

### 2. Auto-inserted sync points hide ambiguities

Auto-inserted sync points (aka `ApplyDeferred`) provide an explicit ordering between systems. This
means that it is possible that two systems are ambiguous, but just so happen to have a sync point
between them which results in an explicit ordering through the sync point, resolving the ambiguity.

The result is that changing systems can result in new ambiguities being made visible even when
neither ambiguous system is being modified. To be clear, these new ambiguities were not a "problem"
before; they could not result in a non-deterministic ordering. However, it is unfortunate that
changing a system can *induce* an ambiguity warning in an unrelated pair of systems. Bevy has
removed any of these situations from itself (we prevent these "strict" ambiguities), so these cases
can only happen where at least one side is a non-Bevy system.

Here is a concrete example:

```rust
fn target() {}
fn ambiguous_1(_: ResMut<R>, _: Commands) {}
fn pusher(_: Commands) {}
fn ambiguous_2(_: ResMut<R>, _: Commands) {}
fn last() {}

fn main() {
  App::new()
    .add_systems(Update, (target, ambiguous_1).chain())
    .add_systems(Update, (pusher, ambiguous_2).chain())
    // This system doesn't really matter to this example, but is technically needed for the error to
    // occur.
    .add_systems(Update, last.after((ambiguous_1, ambiguous_2)))
    .run()
}
```

Because `pusher` includes `Commands`, there must be a sync-point between `pusher` and `ambiguous_2`.
Let's call this sync point `SP`. Since `ambiguous_1` has no commands before it that it needs to
"wait" for, and it itself has commands, it gets an explicit ordering to happen before `SP` (since
that's the first sync-point that is ok to run in). Therefore, since `ambiguous_1` is before `SP`,
and `ambiguous_2` is after `SP`, there is an explicit ordering between them, so there's no
ambiguity.

Now if we mutate `target` to instead be:

```rust
fn target(_: Commands) {}
```

Now, `ambiguous_1` will be ordered *after* `SP` (since `target` uses the sync-point to run its
commands). This results in there being *no ordering* between `ambiguous_1` and `ambiguous_2`, and
therefore we get a warning that there's an ambiguity. The end result is **changing `target` induced
an ambiguity between two other systems**.

Bevy has detected a handful of these cases (and eliminated them). Unfortunately, we do not yet have
a general purpose way to find these cases. If you'd like to find these cases, you can check the
`ambiguity_detection` test in the Bevy repo for our (slightly inconvenient) approach.

## Disabling ambiguity detection

First, seriously consider whether disabling this is the correct action. In some cases, you may start
with a surprising number of ambiguities when migrating. However, after clearing out any initial
ambiguities, new systems are much more likely to have correct ordering (since you will be notified
about any ambiguous orderings).

If you are committed to disabling this, you may do so like this:

```rust
app.configure_schedules(ScheduleBuildSettings {
  ambiguity_detection: LogLevel::Ignore,
  ..Default::default()
});
// If you also want to disable ambiguities in the render app:
app
  .sub_app_mut(bevy_render::RenderApp)
  .configure_schedules(ScheduleBuildSettings {
    ambiguity_detection: LogLevel::Ignore,
    ..Default::default()
  });
```

It's also possible to configure individual schedules to disable ambiguity detection using
`Schedule::set_build_settings`.
