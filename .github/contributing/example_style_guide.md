# Style guide: Examples

For more advice on writing examples, see the [relevant section](/CONTRIBUTING.md#writing-examples) of CONTRIBUTING.md.

## Organization

1. Each example should live in an appropriate subfolder of `/examples` (other than `hello_world.rs`). and take exactly one file unless otherwise needed.
2. Each example should be one file in size if possible.
3. Assets live in `./assets`. Try to avoid adding new assets unless strictly necessary to keep the repo small.
4. Each example should try to follow this order:
   1. Imports
   2. A `fn main()` block
   3. \[Optional\] Constants
   4. Example logic
   5. \[Optional\] Tests
5. Try to structure app / plugin construction in the same fashion as the actual code.

## Stylistic preferences

1. Use simple, descriptive variable names.
   1. Avoid names like `MyComponent` in favor of more descriptive terms like `Events`.
   2. Prefer single letter differentiators like `EventsA` to nonsense words like `EventsFoo`.
   3. Avoid repeating the type of variables in their name where possible. `fn physics` should be preferred to `fn physics_system`, and `Color` should be preferred to `ColorComponent`.
2. Prefer glob imports of `bevy::prelude::*` and `bevy::sub_crate::*` over granular imports.
3. Use a consistent comment style:
   1. `///` doc comments belong above `#[derive(Trait)]` invocations.
   2. `//` comments should generally go above the line in question, rather than in-line.
   3. Avoid `/* */` block comments, even when writing long comments.
   4. Use \`variable_name\` code blocks in comments to signify that you're referring to specific types and variables.
   5. Start comments with capital letters; end them with a period if they are sentence-like.
4. Use comments to organize long and complex stretches of code that can't sensibly be refactored into separate functions.
5. Avoid making variables `pub` unless it is needed for your example.

## Code conventions

1. Refactor configurable values ("magic numbers") out into constants with clear names.
2. Prefer `for` loops over `.for_each`. The latter is faster (for now), but less clear for beginners.
3. Use `.single` and `.single_mut` where appropriate.
4. Prefer `With` filters over unused data-fetching query type arguments.
5. Prefer disjoint queries using `With` and `Without` over query sets when you need more than one query in a single system.
6. Prefer structs with named fields over tuple structs except in the case of single-field wrapper types.

## Feature examples

These examples demonstrate the usage of specific engine features in clear, minimal ways.

1. Try to keep your names divorced from the context of a specific game, and focused on the feature you are demonstrating.
2. Where they exist, show good alternative approaches to accomplish the same task and explain why you may prefer one over the other.
3. Examples should have a visible effect when run, either in the command line or a graphical window.

## Game examples

These examples show how to build simple games in Bevy in a cohesive way.

1. Each of these examples lives in the [/examples/games] folder.
2. Aim for minimum but viable status: the game should be playable and not obviously buggy but does not need to be polished or terribly fun.
3. Focus on code quality, and demonstrating good, extensible patterns for users.
   1. Make good use of enums to organize your game logic.
   2. Keep components as small as possible but no smaller: all of the data on a component should generally be accessed at once.
   3. Keep systems small: they should have a clear single purpose.
   4. Reuse behavior across similar entities using systems whenever possible.
   5. Prefer generics over creating nearly identical types.
4. Use `///` doc comments to explain what each function / struct does as if the example were part of a polished production codebase.
5. Use enum-labels over string-labels for system / stage / etc. labels.
6. Arrange your code into modules within the same file to allow for simple code folding / organization.
