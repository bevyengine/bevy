# Style guide: Engine

## Contributing

For more advice on contributing to the engine, see the [relevant section](../../CONTRIBUTING.md#Contributing-your-own-ideas) of `CONTRIBUTING.md`.

## General guidelines

### Imports

1. Prefer granular imports (eg. `bevy::prelude::App`) over glob imports (eg. `bevy::prelude::*`).

### Documentation

1. Use a consistent comment style:
   1. `///` doc comments belong above `#[derive(Trait)]` invocations.
   2. `//` comments should generally go above the line in question, rather than in-line.
   3. Avoid `/* */` block comments, even when writing long comments.
   4. Use \`variable_name\` code blocks in comments to signify that you're referring to specific types and variables.
   5. Start comments with capital letters. End them with a period if they are sentence-like.
2. Use comments to organize long and complex stretches of code that can't sensibly be refactored into separate functions.

### File structure

#### Library files

The content of a library file (`lib.rs`) should be structured in the following way:

1. crate comment `//!`
2. lints
3. `extern crate`s
4. `pub mod`s
5. `mod`s
6. `pub mod prelude`
7. `pub use`s
8. `use`s
9. `const`s
10. `pub type`s
11. `type`s
12. `pub trait`s
13. `pub struct XyzPlugin`
14. `impl Plugin for x`
15. `struct`s, `enum`s and so on
16. `mod tests`

## Rust API guidelines

As a reference for our API development we are using the [Rust API guidelines][Rust API guidelines]. Generally, these should be followed, except for the following areas of disagreement:

### Areas of disagreements

Some areas mentioned in the [Rust API guidelines][Rust API guidelines] we do not agree with. These areas will be expanded whenever we find something else we do not agree with, so be sure to check these from time to time.

> All items have a rustdoc example

- This guideline is too strong and not applicable for everything inside of the Bevy game engine. For functionality that requires more context or needs a more interactive demonstration (such as rendering or input features), make use of the `examples` folder instead.

> Examples use ?, not try!, not unwrap

- This guideline is usually reasonable, but not always required.

> Only smart pointers implement Deref and DerefMut

- Generally a good rule of thumb, but we're probably going to deliberately violate this for single-element wrapper types like `Life(u32)`. The behavior is still predictable and it significantly improves ergonomics / new user comprehension.

[Rust API guidelines]: https://rust-lang.github.io/api-guidelines/about.html
