---
title: Bevy Error Context Messages
authors: ["@cookie1170"]
pull_requests: [24528]
---

Similar to the popular `anyhow` crate, `BevyError` now provides an ergonomic way to attach extra context to an error using the `context` method,
which also allows creating a `Result<T, BevyError>` from an `Option<T>`.

This makes it easier to trace back errors with human-readable messages without looking at verbose backtraces.

```rs
fn fallible() -> Result<(), BevyError> {
    // This produces the error message `Failed to parse number: invalid digit found in string`
    let parsed: usize = "I am not a number"
        .parse()
        .context("Failed to parse number")?;

    Ok(())
}
```

`with_context` may be used to produce the error string with a closure instead.

If multiple `context`s were used on the same `BevyError`, they're enumerated below:

```rs
fn fallible() -> Result<Package, BevyError> {
    let path = "package.json";
    let package = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {path}"))?;

    serde_json::parse(&package)?
}

fn uses_fallible() -> Result<(), BevyError> {
    let package = fallible().context("Failed to parse package.json")?;
    // Use `package`...
}
```

Will produce the following error if `package.json` is missing:

```rs
Failed to parse package.json

Caused by:
    Failed to read package.json
    No such file or directory (os error 2)
```
