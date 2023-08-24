# Documentations Guide

This guide aims to ensure consistency in the Bevy crates documentation, they should be seen as advice and not strict rules and should therefore be adapted on a case-by-case basis.

## Prelude module's documentations

If the crate has a prelude module, its documentation should be a single sentence with the name of the crate in camel_case and in code format (i.e. between backticks `` `bevy_XXX` ``).

```rust
/// The `bevy_XXX` prelude.
```

Example for `bevy_animation`

```rust
/// The `bevy_animation` prelude.
pub mod prelude {
    /* ... */
}
```

## The crate documentation

1. The first line is a single sentence that summarizes the crate role in the Bevy ecosystem.
2. The first line is followed by an empty line.
3. The documentation may contain more details.
4. Links are placed at the end to clarify documentation reading.
5. The crate attributes (`#![attribute]`) are placed _after_ the documentation.

The format of the documentation will depend on the kind of the crate.

- If the crate adds functionalities to Bevy (e.g. `bevy_time`, `bevy_input`, etc.) the documentation takes this format :

```rust
//! <list of functionality> functionality for the [Bevy game engine].
//!
//! <Optional: rest of the documentation>
//!
//! [Bevy game engine]: https://bevyengine.org/
```

Example for `bevy_input`

```rust
//! Input functionality for the [Bevy game engine].
//!
//! # Supported input devices
//!
//! Bevy currently supports keyboard, mouse, gamepad, and touch inputs.
//!
//! [Bevy game engine]: https://bevyengine.org/
```

- If the crate adapts an external crate for Bevy (e.g. `bevy_winit`, `bevy_gilrs`, etc.) the documentation takes this format :

**Note**: the link to the external crates documentation should point to the version used by the bevy crate and not the latest version.

```rust
//! Integration of the [`XXX`] crate to the [Bevy game engine].
//!
//! <Optional: rest of the documentation>
//!
//! [`XXX`]: <the docs.rs url to the XXX crate>
//! [Bevy game engine]: https://bevyengine.org/
```

Example for `bevy_gilrs`

```rust
//! Integration of the [`gilrs`] crate to the [Bevy game engine].
//!
//! [`gilrs`]: https://docs.rs/gilrs/0.10.1/gilrs/
//! [Bevy game engine]: https://bevyengine.org/
```
