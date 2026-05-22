---
title: "`Command` error handling has been simplified"
pull_requests: [23432, 23477]
---

The `Command` trait now takes `Out` as an associated type rather than as a generic
parameter. For function-style commands that return a `Result`, the code changes as follows:

```rust
// 0.18
fn my_command() -> impl Command<Result> {
    move |world: &mut World| -> Result {
        // ...
    }
}

// 0.19
fn my_command() -> impl Command {
    move |world: &mut World| -> Result {
        // ...
    }
}
```

Implementors of the `Command` trait must now fill in the `Out` associated type:

```rust
// 0.18
impl Command for Foo {
    fn apply(self, world: &mut World) {
        // ...
    }
}

// 0.19
impl Command for Foo {
    type Out = ();

    fn apply(self, world: &mut World) {
        // ...
    }
}
```

For commands that return `Result`:

```rust
// 0.18
impl Command<Result> for Foo {
    fn apply(self, world: &mut World) -> Result {
        // ...
    }
}

// 0.19
impl Command for Foo {
    type Out = Result;

    fn apply(self, world: &mut World) -> Result {
        // ...
    }
}
```

The functionality of the `HandleError` and `CommandWithEntity` traits have been
folded into `Command` and `EntityCommand`, respectively. Use the latter traits as
the sole trait bound, if needed.
