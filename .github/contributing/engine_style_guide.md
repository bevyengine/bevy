# Style guide: Engine

## Contributing

For more advice on contributing to the engine, see the [relevant section](../../CONTRIBUTING.md#Contributing-your-own-ideas) of `CONTRIBUTING.md`.

## General guidelines

1. Prefer granular imports over glob imports of `bevy::prelude::*` and `bevy::sub_crate::*`.
2. Use a consistent comment style:
   1. `///` doc comments belong above `#[derive(Trait)]` invocations.
   2. `//` comments should generally go above the line in question, rather than in-line.
   3. Avoid `/* */` block comments, even when writing long comments.
   4. Use \`variable_name\` code blocks in comments to signify that you're referring to specific types and variables.
   5. Start comments with capital letters. End them with a period if they are sentence-like.
3. Use comments to organize long and complex stretches of code that can't sensibly be refactored into separate functions.

## Rust API guidelines

As a reference for our API development we are using the [Rust API guidelines][Rust API guidelines]. These should be respected and followed except for the following areas of disagreements.

### Areas of disagreements

Some areas mentioned in the [Rust API guidelines][Rust API guidelines] we do not agree with. These areas will be expanded whenever we find something else we do not agree with, so be sure to check these from time to time.

> All items have a rustdoc example
- This guideline is too strong and not applicable for everything inside of the Bevy game engine. Instead we should make use of the `examples` folder and add new examples that way if needed.
> Examples use ?, not try!, not unwrap
- This guideline is usually reasonable, but not always required.
> Only smart pointers implement Deref and DerefMut
- Generally a good rule of thumb, but we're probably going to deliberately violate this for wrapper types due to the lack of any better ergonomic solution.

[Rust API guidelines]: https://rust-lang.github.io/api-guidelines/about.html
