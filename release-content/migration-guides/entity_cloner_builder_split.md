---
title: EntityClonerBuilder Split
pull_requests: [19649, 19977]
---

`EntityClonerBuilder` is now generic and has different methods depending on the generic.

To get the wanted one, `EntityCloner::build` got split too:

- `EntityCloner::build_opt_out` to get `EntityClonerBuilder<OptOut>`
- `EntityCloner::build_opt_in` to get `EntityClonerBuilder<OptIn>`

The first is used to clone all components possible and optionally _opting out_ of some.
The second is used to only clone components as specified by _opting in_ for them.

```rs
// 0.16
let mut builder = EntityCloner.build(&mut world);
builder.allow_all().deny::<ComponentThatShouldNotBeCloned>();
builder.clone_entity(source_entity, target_entity);

let mut builder = EntityCloner.build(&mut world);
builder.deny_all().allow::<ComponentThatShouldBeCloned>();
builder.clone_entity(source_entity, target_entity);

// 0.17
let mut builder = EntityCloner.build_opt_out(&mut world);
builder.deny::<ComponentThatShouldNotBeCloned>();
builder.clone_entity(source_entity, target_entity);

let mut builder = EntityCloner.build_opt_in(&mut world);
builder.allow::<ComponentThatShouldBeCloned>();
builder.clone_entity(source_entity, target_entity);
```

Still, using `EntityClonerBuilder::finish` will return a non-generic `EntityCloner`.
This change is done because the behavior of the two is too different to share the same struct and same methods and mixing them caused bugs.

The methods of the two builder types are different to 0.16 and to each other now:

## Opt-Out variant

- Still offers variants of the `deny` methods.
- No longer offers `allow` methods, you need to be exact with denying components.
- Offers now the `insert_mode` method to configure if components are cloned if they already exist at the target.
- Required components of denied components are no longer considered. Denying `A`, which requires `B`, does not imply `B` alone would not be useful at the target. So if you do not want to clone `B` too, you need to deny it explicitly. This also means there is no `without_required_components` method anymore as that would be redundant.
- It is now the other way around: Denying `A`, which is required _by_ `C`, will now also deny `C`. This can be bypassed with the new `without_required_by_components` method.

## Opt-In variant

- Still offers variants of the `allow` methods.
- No longer offers `deny` methods, you need to be exact with allowing components.
- Offers now `allow_if_new` method variants that only clone this component if the target does not contain it. If it does, required components of it will also not be cloned, except those that are also required by one that is actually cloned.
- Still offers the `without_required_components` method.

## Common methods

All other methods `EntityClonerBuilder` had in 0.16 are still available for both variants:

- `with_default_clone_fn`
- `move_components`
- `clone_behavior` variants
- `linked_cloning`

## Unified id filtering

Previously `EntityClonerBuilder` supported filtering by 2 types of ids: `ComponentId` and `TypeId`, the functions taking in `IntoIterator` for them.
Since now `EntityClonerBuilder` supports filtering by `BundleId` as well, the number of method variations would become a bit too unwieldy.
Instead, all id filtering methods were unified into generic `deny_by_ids/allow_by_ids(_if_new)` methods, which allow to filter components by
`TypeId`, `ComponentId`, `BundleId` and their `IntoIterator` variations.

## Other affected APIs

| 0.16 | 0.17 |
| - | - |
| `EntityWorldMut::clone_with` | `EntityWorldMut::clone_with_opt_out` `EntityWorldMut::clone_with_opt_in` |
| `EntityWorldMut::clone_and_spawn_with` | `EntityWorldMut::clone_and_spawn_with_opt_out` `EntityWorldMut::clone_and_spawn_with_opt_in` |
| `EntityCommands::clone_with` | `EntityCommands::clone_with_opt_out` `EntityCommands::clone_with_opt_in` |
| `EntityCommands::clone_and_spawn_with` | `EntityCommands::clone_and_spawn_with_opt_out` `EntityCommands::clone_and_spawn_with_opt_in` |
| `entity_command::clone_with` | `entity_command::clone_with_opt_out` `entity_command::clone_with_opt_in` |
