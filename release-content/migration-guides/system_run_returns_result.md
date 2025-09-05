---
title: "`System::run` returns `Result`"
pull_requests: [19145]
---

In order to support fallible systems and parameter-based system skipping like `Single` and `If<T>` in more places, `System::run` and related methods now return a `Result` instead of a plain value.

If you were calling `System::run`, `System::run_unsafe`, `System::run_without_applying_deferred`, or `ReadOnlySystem::run_readonly`, the simplest solution is to `unwrap()` the resulting `Result`.
The only case where an infallible system will return `Err` is an invalid parameter, such as a missing resource, and those cases used to panic.

If you were calling them from a function that returns `Result<T, BevyError>`, you can instead use the `?` operator.

`System::run`, `System::run_without_applying_deferred`, and `ReadOnlySystem::run_readonly` will now call `System::validate_param_unsafe` and return `Err` if validation fails.
If you were calling `validate_param` or `validate_param_unsafe` before calling one of those, it is no longer necessary.
Note that `System::run_unsafe` still does *not* perform validation.

If you were manually implementing `System`, the return type to `run_unsafe` has changed from `Out` to `Result<Out, RunSystemError>`.
If you are implementing an infallible system, simply wrap the return value in `Ok`.
If you were implementing a fallible system and had set `type Out = Result<T, BevyError>;`, instead set `type Out = T;`.

If you have a system function that returns `Result` or `!` and are not restricting the return type, you may get type inference failures like this:

```text
error[E0283]: type annotations needed
    --> lib.rs:100:5
     |
100  |     IntoSystem::into_system(example_system);
     |     ^^^^^^^^^^^^^^^^^^^^^^^ cannot infer type of the type parameter `Out` declared on the trait `IntoSystem`
     |
note: multiple `impl`s satisfying `core::result::Result<(), bevy_error::BevyError>: function_system::IntoResult<_>` found
   --> crates\bevy_ecs\src\system\function_system.rs:597:1
```

or

```text
error[E0283]: type annotations needed
    --> lib.rs:100:11
    |
100 |     world.run_system_cached(system).unwrap();
    |           ^^^^^^^^^^^^^^^^^ cannot infer type of the type parameter `O` declared on the method `run_system_cached`
    |
note: multiple `impl`s satisfying `core::result::Result<(), bevy_error::BevyError>: function_system::IntoResult<_>` found
   --> crates\bevy_ecs\src\system\function_system.rs:597:1
```

A function that returns `Result<T, BevyError>` may be considered either a fallible system that returns `T` or an infallible system that returns `Result`, and a function that returns `!` may be considered a system that returns *any* type.
You should be able to resolve them by providing an explicit type for `System::Out`.

```rust
fn example_system() -> Result { Ok(()) }
// 0.16 - Output type is inferred to be `Result`
IntoSystem::into_system(example_system)
// 0.17 - Output type can be either `()` or `Result` and must be written explicitly
IntoSystem::<_, (), _>::into_system(example_system);
IntoSystem::<_, Result, _>::into_system(example_system);

// 0.16 - Output type is inferred to be `Result`
world.run_system_cached(example_system).unwrap().unwrap();
// 0.17 - Output type can be either `()` or `Result` and must be written explicitly
world.run_system_cached::<(), _, _>(example_system).unwrap();
world.run_system_cached::<Result, _, _>(example_system).unwrap().unwrap();
// or it may be inferred if the output type is specified elsewhere
let _: () = world.run_system_cached(example_system).unwrap();
let r: Result = world.run_system_cached(example_system).unwrap();
```
