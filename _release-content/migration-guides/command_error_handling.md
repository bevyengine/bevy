---
title: "`Command` error handling has been simplified"
pull_requests: [23432, 23477]
---

The `Command` trait now takes `Out` as an associated type rather than as a generic
parameter. For function-style commands that return a `Result`, the following change must be made:

```rust
// Before
fn my_command() -> impl Command<Result> {
    move |world: &mut World| -> Result {
        // ...
    }
}

// After
fn my_command() -> impl Command {
    move |world: &mut World| -> Result {
        // ...
    }
}
```

Implementors of the `Command` trait must now fill in the `Out` associated type:

```rust
// Before
impl Command for Foo {
    fn apply(self, world: &mut World) {
        // ...
    }
}

// After
impl Command for Foo {
    type Out = ();

    fn apply(self, world: &mut World) {
        // ...
    }
}
```

For commands that return `Result`:

```rust
// Before
impl Command<Result> for Foo {
    fn apply(self, world: &mut World) -> Result {
        // ...
    }
}

// After
impl Command for Foo {
    type Out = Result;

    fn apply(self, world: &mut World) -> Result {
        // ...
    }
}
```

The functionality of the `HandleError` and `CommandWithEntity` traits have been
folded into `Command` and `EntityCommand`, respectively. Use the latter traits as
the sole trait bound, as needed.
