---
title: EntityClonerBuilder Split
pull_requests: [19649]
---

`EntityClonerBuilder` is now generic and has different methods depending on the generic.

To get the wanted one, `EntityCloner::build` got split too:
- `EntityCloner::build_opt_in` to get `EntityClonerBuilder<OptIn>`
- `EntityCloner::build_opt_out` to get `EntityClonerBuilder<OptOut>`

The first is used to clone all components possible and optionally _opt out_ of some.
The second is used to only clone components as specified by _opt in_.

```rs
// 0.16
let mut builder = EntityCloner.build(&mut world);
builder.allow_all().deny::<DoNotCloneComponent>();
builder.clone_entity(source_entity, target_entity);

let mut builder = EntityCloner.build(&mut world);
builder.deny_all().allow::<DoCloneComponent>();
builder.clone_entity(source_entity, target_entity);

// 0.17
let mut builder = EntityCloner.build_opt_out(&mut world);
builder.deny::<DoNotCloneComponent>();
builder.clone_entity(source_entity, target_entity);

let mut builder = EntityCloner.build_opt_in(&mut world);
builder.allow::<DoCloneComponent>();
builder.clone_entity(source_entity, target_entity);
```

Still, using `EntityClonerBuilder::finish` will return a non-generic `EntityCloner`.
This is done because the behavior of the two is too different to share the same struct and same methods and mixing them caused bugs.

As that means there are now two builder types, their API and behavior is different to each other:

## Opt-Out variant

- Still offers variants of the `deny` methods which now also includes one with a `BundleId` argument.
- No longer offers `allow` methods, you need to be exact with denying components.
- Offers now the `insert_mode` method to configure if components are cloned if they already exist at the target.
- Does not consider required components anymore. Denying `A`, which requires `B`, does not imply `B` alone would not be useful at the target. So if you do not want to clone `B` too, you need to deny it explicitly.
- Because of the previous bullet point, no longer offers the `without_required_components` method as that would be redundant.

## Opt-In variant

- Still offers variants of the `allow` methods which now also includes one with a `BundleId` argument.
- No longer offers `deny` methods, you need to be exact with allowing components.
- Offers now `allow_if_new` method variants that only clone this component if the target does not contain it. If it does, required components of it will also not be cloned, unless it is also required of one that is actually cloned.
- Still offers the `without_required_components` method.

## Common methods

All other methods `EntityClonerBuilder` had in 0.16 are still available for both variants:
- `with_default_clone_fn`
- `move_components`
- `clone_behavior` variants
- `linked_cloning`

## Other affected APIs

| 0.16 | 0.17 |
| - | - |
| `EntityWorldMut::clone_with` | `EntityWorldMut::clone_with_opt_out` <br> `EntityWorldMut::clone_with_opt_in` |
| `EntityWorldMut::clone_and_spawn_with` | `EntityWorldMut::clone_and_spawn_with_opt_out` <br> `EntityWorldMut::clone_and_spawn_with_opt_in` |
| `EntityCommands::clone_with` | `EntityCommands::clone_with_opt_out` <br> `EntityCommands::clone_with_opt_in` |
| `EntityCommands::clone_and_spawn_with` | `EntityCommands::clone_and_spawn_with_opt_out` <br> `EntityCommands::clone_and_spawn_with_opt_in` |
| `entity_command::clone_with` | `entity_command::clone_with_opt_out`<br>`entity_command::clone_with_opt_in` |
