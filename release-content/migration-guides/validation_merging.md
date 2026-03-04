---
title: "`SystemParam` validation is now done when fetching the data"
pull_requests: [TODO]
---

In an effort to improve performance by reducing redundant data fetches and simplify internals,
system parameter validation is now done as part of fetching the data for those system parameters.
To be more precise:

- `SystemParam::get_param` now returns a `Result<Self::Item<'world, 'state>, SystemParamValidationError>`, instead of simply a `Self::Item<'world, 'state>`
  - If validation fails, an appropriate `SystemParamValidationError` should be returned
  - If validation passes, the item should be returned wrapped in `Ok`
- `SystemParam::validate_params` has been removed
  - All logic that's currently done in this methods should be moved to the `get_param` method of that type

When executing systems, we no longer check for system validation before running the systems.
As a result of these changes, `System::validate_param` and `System::validate_param_unsafe` have been removed.
Instead, validation has been moved to be part of the trait implementation for `System::run_unsafe`.
All implementations of the `System` should validate that their parameters are valid during this method,
bubbling up any errors originating in `SystemParam::get_param`.

For the parallel `MultithreadedExecutor`, validation was previously done while checking run conditions.
Now, tasks will be spawned for systems which would fail or are skipped during validation.
If you find that this is a significant performance overhead for your use case,
the previous behavior can be recovered by adding run conditions.
