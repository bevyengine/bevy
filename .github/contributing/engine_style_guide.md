# Style guide: Engine

For more advice on contributing to the engine, see the [relevant section](../../CONTRIBUTING.md#Contributing-your-own-ideas) of CONTRIBUTING.md.

1. Prefer granular imports over glob imports of `bevy::prelude::*` and `bevy::sub_crate::*`.
2. Use a consistent comment style:
   1. `///` doc comments belong above `#[derive(Trait)]` invocations.
   2. `//` comments should generally go above the line in question, rather than in-line.
   3. Avoid `/* */` block comments, even when writing long comments.
   4. Use \`variable_name\` code blocks in comments to signify that you're referring to specific types and variables.
   5. Start comments with capital letters. End them with a period if they are sentence-like.
3. Use comments to organize long and complex stretches of code that can't sensibly be refactored into separate functions.
