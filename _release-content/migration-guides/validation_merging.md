---
title: "`SystemParam` validation is now done when fetching the data"
pull_requests: [23225]
---

In an effort to improve performance by reducing redundant data fetches and simplify internals,
system parameter validation is now done as part of fetching the data for those system parameters.
To be more precise:

- `SystemParam::get_param` now returns a `Result<Self::Item<'world, 'state>, SystemParamValidationError>`, instead of simply a `Self::Item<'world, 'state>`
  - If validation fails, an appropriate `SystemParamValidationError` should be returned
  - If validation passes, the item should be returned wrapped in `Ok`
- `SystemParam::validate_param` has been removed
  - All logic that was done in this method should be moved to the `get_param` method of that type
- `SystemState::validate_param` has been removed
  - Validation now happens automatically when calling `get`, `get_mut`, or `get_unchecked`
- `SystemState::fetch`, `get_unchecked`, `get` and `get_mut` now return a `Result<..., SystemParamValidationError>`. Callers that previously destructured the result directly will need to add `.unwrap()` or handle the `Result`:

```rust
// Before
let (res, query) = system_state.get(&world);

// After
let (res, query) = system_state.get(&world).unwrap();
```

When executing systems, we no longer check for system validation before running the systems.
As a result of these changes, `System::validate_param` and `System::validate_param_unsafe` have been removed.
Instead, validation has been moved to be part of the trait implementation for `System::run_unsafe`.
All implementations of the `System` trait should validate that their parameters are valid during this method,
bubbling up any errors originating in `SystemParam::get_param`.

## Custom `SystemParam` implementations

If you have a custom `SystemParam` implementation, you need to:

1. Remove the `validate_param` method.
2. Move any validation logic into `get_param`.
3. Change `get_param` to return `Result<Self::Item<'world, 'state>, SystemParamValidationError>`.

```rust
// Before
unsafe impl SystemParam for MyParam<'_> {
    // ...
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // validation logic
        if !is_valid(state, world) {
            return Err(SystemParamValidationError::invalid::<Self>("not valid"));
        }
        Ok(())
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // fetch logic
        MyParam { /* ... */ }
    }
}

// After
unsafe impl SystemParam for MyParam<'_> {
    // ...
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        // validation logic merged into get_param
        if !is_valid(state, world) {
            return Err(SystemParamValidationError::invalid::<Self>("not valid"));
        }
        // fetch logic
        Ok(MyParam { /* ... */ })
    }
}
```

## Custom `ExclusiveSystemParam` implementations

Similarly, `ExclusiveSystemParam::get_param` now returns a `Result<Self::Item<'s>, SystemParamValidationError>` instead of `Self::Item<'s>`.
Existing implementations should wrap their return value in `Ok(...)` and return an appropriate `SystemParamValidationError` if validation fails.

```rust
// Before
impl ExclusiveSystemParam for MyExclusiveParam {
    // ...
    fn get_param<'s>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
    ) -> Self::Item<'s> {
        MyExclusiveParam { /* ... */ }
    }
}

// After
impl ExclusiveSystemParam for MyExclusiveParam {
    // ...
    fn get_param<'s>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
    ) -> Result<Self::Item<'s>, SystemParamValidationError> {
        Ok(MyExclusiveParam { /* ... */ })
    }
}
```

## Custom `System` implementations

If you have a custom `System` implementation, remove the `validate_param_unsafe` method. Parameter validation should now occur inside `run_unsafe` by propagating errors from `SystemParam::get_param`.

## `MultithreadedExecutor` performance changes

For the parallel `MultithreadedExecutor`, validation was previously done as a cheap pre-validation step,
while checking run conditions.
Now, tasks will be spawned for systems which would fail or are skipped during validation.

In most cases, avoiding the extra overhead of looking up the required data twice should dominate.
However, this change may negatively affect systems which are frequently skipped (e.g. due to `Single`).
If you find that this is a significant performance overhead for your use case,
the previous behavior can be recovered by adding run conditions.
