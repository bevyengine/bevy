---
title: Better system access conflict errors
pull_requests: [25507, 25731]
---

To provide better error messages,
system access conflicts are now managed by returning `Result` instead of panicking.

The signatures of `SystemParam::init_access` and `WorldQuery::init_nested_access`
have been changed to return `Result`, and to remove the `world` and `system_name`
parameters that were only used to build the panic message.
Manual implementations and any manual calls will need to be changed.

Manual implementations of `SystemParam` or `WorldQuery` that
delegate to other implementations should propagate the `Err`,
either by returning the value directly or using the `?` operator.

Note that propagating the error will include the type name
of the *inner* parameter in the panic message.
Use `map_err(ParameterAccessConflict::with_param::<Self>)`
to replace it with the wrapper's type name, if desired.

Implementations that perform no access can simply return `Ok(())`.
Note that performing metadata access using safe methods on `UnsafeWorldCell`
requires registering metadata access to detect conflicts with `&mut World`.
Use `SystemAccess::try_extend_metadata` if you call such methods in `get_param`.

Implementations that need to register custom access
should call one of the `SystemAccess::try_extend` methods to try to add the access,
and then call `ParameterAccessConflict::new::<Self>` on an `Err`.

```rust
impl SystemParam for ExampleParameter {
    // 0.19
    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        let mut access: FilteredAccess = ...;
        let conflicts = component_access_set.get_conflicts_single(&access);
        if !conflicts.is_empty() {
            panic!("...");
        }
        component_access_set.add(access);
    }

    // 0.20
    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        system_access: &mut SystemAccess,
    ) -> Result<(), Box<ParameterAccessConflict>> {
        let mut access: FilteredAccess = ...;
        system_access.try_extend_single(access).map_err(|access| {
            ParameterAccessConflict::new::<Self>(access)
                // If you have additional suggestions to display to the user on conflict,
                // call `with_suggestion` or `with_suggestion_if_exclusive` to add them.
                .with_suggestion("Suggestion on conflict")
                .with_suggestion_if_exclusive(system_access, "Suggestion on conflict with `&mut World`")
        })
    }
}
```
