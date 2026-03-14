---
title: "`SystemParam` validation is now done when fetching the data"
pull_requests: [23225]
---

In an effort to improve performance by reducing redundant data fetches and simplify internals,
system parameter validation is now done as part of fetching the data for those system parameters.
To be more precise:

- `SystemParam::get_param` has been renamed `try_param` and now returns a `Result<Self::Item<'world, 'state>, SystemParamValidationError>`, instead of simply a `Self::Item<'world, 'state>`
  - If validation fails, an appropriate `SystemParamValidationError` should be returned
  - If validation passes, the item should be returned wrapped in `Ok`
- A new `InfallibleSystemParam` trait has been added for parameters like `Query` that are always valid.  This has a `get_param` method that returns a `Self::Item<'world, 'state>`.
- `SystemParam::validate_param` has been removed
  - All logic that was done in this method should be moved to the `get_param` method of that type
- `SystemState::validate_param` has been removed
  - Validation now happens automatically when calling `get`, `get_mut`, or `get_unchecked`
- `SystemState::fetch`, `get_unchecked`, `get` and `get_mut` now require `InfallibleSystemParam`.  For fallible parameters, there are now `try_` variants that return a `Result<..., SystemParamValidationError>`. Callers that previously destructured the result of a fallible parameter directly will need to call `try_get` and add `.unwrap()` or handle the `Result`:

```rust
// Before
let (res, query) = system_state.get(&world);

// After
let (res, query) = system_state.try_get(&world).unwrap();
```

- Similarly, the `pN()` family of methods on `ParamSet` now require `InfallibleSystemParam`, and `try_pN()` methods have been added that return a `Result`.
- `ParamSet` subparameters are no longer validated before running the system, so a system with a `ParamSet` containing a `Single` will now run.  To have the system skip, ensure it returns `Result<(), RunSystemError>` and replace `pN()` with `try_pN()?`, where `N` is the index of the `Single`.

```rust
// Before
fn system(param_set: ParamSet<(Query<&mut T>, Single<&mut T, With<U>)>) {
    // This will not run at all if there would be no matching entity
    do_something();
    let t = param_set.p1();
}

// After
fn system(param_set: ParamSet<(Query<&mut T>, Single<&mut T, With<U>)>) -> Result<(), RunSystemError> {
    // This will always run, because we have not yet validated the `Single`.
    do_something();
    let t = param_set.try_p1()?;
}
```

If you get a compiler error like ``the trait bound `SomeType: InfallibleSystemParam` is not satisfied``
on a call to `SystemState::get()` or `ParamSet::pN()`,
switch to `try_get` or `try_pN` as described above.

If the type is defined using `#[derive(SystemParam)]` and only includes infallible parameters,
you may instead `#[derive(SystemParam, InfallibleSystemParam)]`.

If you were using `ParamSet<(MessageReader<M>, MessageWriter<M>)>` to send and receive messages in the same system,
you may instead use `MessageMutator<M>`, which can read and write messages without needing `ParamSet` at all.

When executing systems, we no longer check for system validation before running the systems.
As a result of these changes, `System::validate_param` and `System::validate_param_unsafe` have been removed.
Instead, validation has been moved to be part of the trait implementation for `System::run_unsafe`.
All implementations of the `System` trait should validate that their parameters are valid during this method,
bubbling up any errors originating in `SystemParam::get_param`.

## Custom `SystemParam` implementations

If you have a custom `SystemParam` implementation, you need to either:

If `validate_param` was overridden, because the parameter was sometimes invalid:

1. Remove the `validate_param` method.
2. Move any validation logic into `get_param`.
3. Rename `get_param` to `try_get_param` and return `Result<Self::Item<'world, 'state>, SystemParamValidationError>`.

If `validate_param` was not overridden, because the parameter was always valid:

1. Add an `InfallibleSystemParam` impl
2. Move `get_param` to that impl
3. Create a `SystemParam::try_get_param` method that calls `get_param`

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

// After, for fallible parameters
unsafe impl SystemParam for MyParam<'_> {
    // ...
    unsafe fn try_get_param<'w, 's>(
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

// After, for infallible parameters
unsafe impl SystemParam for MyParam<'_> {
    // ...
    unsafe fn try_get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        // SAFETY: `try_get_param` has the same safety requirements as `get_param`
        Ok(unsafe { Self::get_param(state, system_meta, world, change_tick) })
    }
}

impl InfallibleSystemParam for MyParam<'_> {
    // ...
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
    fn try_get_param<'s>(
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
