mod bsn;
mod scene_component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Creates a `Scene` using BSN (Bevy Scene Notation) syntax.
///
/// BSN is a concise DSL for defining Bevy scenes as hierarchical collections
/// of entities and components.
/// While BSN's syntax largely follows Rust, it has quite a few features.
/// Don't feel like you need to master all of it before you begin.
/// Start simple and check back on the documentation as you run into problems.
///
/// Trying to decipher a strange combination of glyphs? Jump to the **Syntax Reference** section below.
///
/// ## Basic usage
///
/// Let's begin by spawning a single named entity with a couple of components:
///
/// ```rust, ignore
/// #[derive(Component, Default, Clone)]
/// struct Score(u32);
///
/// #[derive(Component, FromTemplate)]
/// struct Health { current: u32, max: u32 }
///
/// // Spawns a single entity named "Player" with a Score and Health component.
/// world.spawn_scene(bsn! {
///     #Player
///     Score(0)
///     Health { current: 100, max: 100 }
/// });
/// ```
///
/// Each `bsn!` block describes a single root entity.
///
/// ## Children and relationships
///
/// You can add child entities with the `Children` component.
/// To begin a new child entity, separate it from the previous one with a comma.
/// Separate components on the same entity with whitespace.
///
/// ```rust, ignore
/// // ONE child entity with both the Head and Body components:
/// bsn! {
///     #Player
///     Life
///     Children [Head Body]
/// }
///
/// // TWO child entities — one with Head, one with Body:
/// bsn! {
///     #Player
///     Life
///     Children [Head, Body]
/// }
/// ```
///
/// **Warning:** Separating items with whitespace places them on the **same** entity;
/// separating them with commas creates **separate** entities.
/// This is a common source of problems for new BSN users,
/// so be sure to check your commas if an entity seems to be missing!
///
/// Child entities can themselves have components and children, so scenes nest arbitrarily deep:
///
/// ```rust, ignore
/// bsn! {
///     #Town
///     Children [
///         // Parentheses and indentation help clarify the structure of nested scenes,
///         // but are optional — this scene would be the same without them.
///         (
///             #Tavern
///             Children [
///                 #Innkeeper,
///                 #Barkeep,
///             ]
///         ),
///         (
///             #Blacksmith
///             Children [
///                 #Anvil,
///             ]
///         ),
///     ]
/// }
/// ```
///
/// Swap `Children` for any `RelationshipTarget` component to define custom relationships.
///
/// To attach entities to an *existing* entity not created by this scene, use the `ChildOf` component directly.
/// The following example uses [`bsn_list!`] to spawn two entities both attached to a pre-existing root:
///
/// ```rust, ignore
/// let root: Entity = /* some pre-existing entity */;
/// bsn_list! {
///     ( #Child1 ChildOf(root) ),
///     ( #Child2 ChildOf(#Child1) ),
/// }
/// ```
///
/// `ChildOf` accepts either a plain `Entity` value or a named `#Name` reference from the same scope.
///
/// ## Named entity references
///
/// The `#Name` syntax does two things at once: it adds a `Name("Name")` component to the entity
/// and registers it so that other entities in the same scene scope can refer to it by name.
/// To use a named entity as a value — for example, as a component field that holds an `Entity` id —
/// write `#Name` in the value position:
///
/// ```rust, ignore
/// #[derive(Component, FromTemplate)]
/// struct Link(Entity);
///
/// // Spawn a parent named "Hub" with two children that each hold a back-reference to it.
/// bsn! {
///     #Hub
///     Children [
///         Link(#Hub),
///         Link(#Hub),
///     ]
/// }
/// ```
///
/// References can point in any direction within the scene: a parent can reference a descendant,
/// a child can reference a parent, and siblings within the same `Children [...]` block can
/// reference each other.
/// All names in a single `bsn!` call share one scope; names from composed or inherited scenes
/// (`my_scene()`, `:my_scene`) live in their own separate scopes and are not visible here.
/// If two scopes both define the same name (e.g. both use `#Player`), each `#Player` resolves
/// to its own entity — there is no conflict or shadowing.
///
/// For dynamic names computed at runtime, use `#{expr}`:
///
/// ```rust, ignore
/// fn reference_named_entity(name: &str) -> impl Scene {
///     bsn! { #{name} }
/// }
/// ```
///
/// In [`bsn_list!`], all root entries share one scope, so sibling root entities can
/// cross-reference each other — see the `bevy_scene` crate docs for more details.
///
/// ## Defaults and patching
///
/// BSN supports *patching*: writing `Health { current: 100 }` creates a patch that sets only
/// `current`. Unmentioned fields keep their values from earlier patches or the type's defaults,
/// and multiple patches to the same component merge rather than overwrite. This works for both
/// `Clone + Default` and `FromTemplate` types.
///
/// The difference between the two is about what values a field can hold at spawn time:
///
/// - **`Clone + Default` types** (e.g. `#[derive(Component, Default, Clone)]`): the simple
///   case. This just works in BSN with no extra derives. All field values must be plain Rust values — the
///   template cannot fill them in correctly based on world or context state.
///
/// - **`FromTemplate` types** (e.g. `#[derive(Component, FromTemplate)]`): needed when a field
///   requires spawn-time context. Use this when a field's type itself implements `FromTemplate`
///   — for example, `Handle<T>` fields that resolve asset path strings, or `Entity` fields that
///   reference named entities in the scene.
///
/// Because each approach generates a different `Template` implementation, `Clone + Default` and
/// `FromTemplate` cannot both be derived on the same type. This would create incoherent trait implementations!
///  Use `Clone + Default` by default, and switch to `FromTemplate` only when you need the extra flexibility it provides.
///
/// ## Expressions and dynamic values
///
/// BSN supports embedding arbitrary Rust expressions anywhere a value is expected,
/// using `{...}` (curly braces):
///
/// ```rust, ignore
/// fn enemy(name: &str, hp: u32) -> impl Scene {
///     let sprite_path = name.to_string() + ".png";
///     bsn! {
///         #{name}
///         Health { current: {hp}, max: {hp} }
///         Sprite { image: {sprite_path} }
///     }
/// }
/// ```
///
/// A `{...}` block can also hold an expression that implements `Scene` or, inside a
/// `Children [...]` block, one that implements `SceneList`:
///
/// ```rust, ignore
/// fn unit_with_armor(unit_base: impl Scene) -> impl Scene {
///     bsn! {
///         {unit_base}
///         Armor(50)
///     }
/// }
/// ```
///
/// **Note:** `.bsn` asset files will not support arbitrary Rust expressions,
/// as we do not intend to require Bevy games to ship a Rust compiler.
///
/// ## Automatic type conversion
///
/// BSN performs some automatic type conversion for you,
/// reducing boilerplate when creating scenes.
///
/// The `bsn!` macro appends `.into()` to expressions (`{...}`), bare identifiers, closures, and string
/// literals when they appear as field values. This means any field assignment that would be
/// valid via Rust's standard [`From`]/[`Into`] traits works transparently in BSN — no explicit
/// cast needed:
///
/// ```rust, ignore
/// #[derive(Component, Default, Clone)]
/// struct Label(String);
///
/// let greeting: &'static str = "Hello";
/// bsn! {
///     // &str → String via Into, no .to_string() required
///     Label(greeting)
/// }
/// ```
///
/// A related, more advanced trick is what makes string literals work as asset paths for `Handle<T>` fields.
/// See the docs on `HandleTemplate` for more information!
///
/// Note: non-string literals are used as-is. Only string literals get `.into()` appended;
/// all other literals (integers, floats, booleans) are emitted directly,
/// so `Health { current: 100 }` assigns `100` without any conversion.
/// If the types don't match (e.g. you forgot to append a decimal point to a float),
/// you'll get a normal Rust type error.
///
/// ## Asset loading
///
/// When a component field is a `Handle<T>`, BSN accepts a string literal in its place.
/// The string is resolved to an asset handle at spawn time via the asset server, reusing
/// an existing handle if the asset is already loaded:
///
/// ```rust, ignore
/// commands.spawn_scene(bsn! {
///     Sprite { image: "player.png" }
/// });
/// ```
///
/// This works for your own `FromTemplate`-derived components too — any `Handle<T>` field
/// automatically accepts asset path strings:
///
/// ```rust, ignore
/// #[derive(Component, FromTemplate)]
/// struct Icon {
///     image: Handle<Image>,
///     tint: Color,
/// }
///
/// commands.spawn_scene(bsn! {
///     Icon { image: "icon.png", tint: Color::WHITE }
/// });
/// ```
///
/// ## Observers
///
/// Use `on` inside `bsn!` to attach an entity observer — a closure that fires when a given
/// `EntityEvent` targets the entity. The first parameter's type determines which event is
/// observed. Multiple observers can be stacked on the same entity, and the closure has full
/// access to the ECS via system parameters:
///
/// ```rust, ignore
/// #[derive(EntityEvent)]
/// struct Damage(u32);
///
/// fn player() -> impl Scene {
///     bsn! {
///         Health { max: 100, current: 100 }
///         on(|ev: On<Damage>, mut query: Query<&mut Health>| {
///             let mut health = query.get_mut(ev.target()).unwrap();
///             health.current = health.current.saturating_sub(ev.0);
///         })
///     }
/// }
///
/// // `move` closures work too, capturing variables from the enclosing scope:
/// fn enemy(bonus_damage: u32) -> impl Scene {
///     bsn! {
///         on(move |ev: On<Damage>, mut query: Query<&mut Health>| {
///             let mut health = query.get_mut(ev.target()).unwrap();
///             health.current = health.current.saturating_sub(ev.0 + bonus_damage);
///         })
///     }
/// }
/// ```
///
/// ## Scene composition and inheritance
///
/// There are two ways to build on an existing scene: **inline composition** and **inheritance**.
///
/// **Inline composition** calls a scene function directly inside `bsn!`. The parent's unresolved
/// templates are merged with the child's and everything resolves together in one pass:
///
/// ```rust, ignore
/// fn base_enemy() -> impl Scene {
///     bsn! {
///         Health { current: 100, max: 100 }
///         Power(10)
///     }
/// }
///
/// // Compose base_enemy() and patch just the fields that differ.
/// fn boss() -> impl Scene {
///     bsn! {
///         base_enemy()
///         Health { max: 500 }
///         Power(50)
///     }
/// }
/// ```
///
/// **Inheritance** uses the `:` prefix. The parent is *pre-resolved* first — its templates are
/// fully flattened into a `ResolvedScene` — and the child's patches are applied on top.
/// When the scene is parameterless, this will "cache" the scene and share it across all inheriting scenes.
/// For larger scenes that are inherited many times, this can be much faster than re-computing
/// the scene each time.
///
/// ```rust, ignore
/// fn boss() -> impl Scene {
///     bsn! {
///         :base_enemy
///         Health { max: 500 }
///         Power(50)
///     }
/// }
///
/// // Asset inheritance (.bsn format not yet released, sorry!):
/// bsn! {
///     :"enemy.bsn"
///     Health { max: 500 }
/// }
/// ```
///
/// |                           | Function inheritance `:my_scene`    | Asset inheritance `:"my_scene.bsn"` | Inline composition `my_scene()`  |
/// |---------------------------|-------------------------------------|-------------------------------------|----------------------------------|
/// | Accepts parameters        | Yes                                 | No                                  | Yes                              |
/// | Asset-based               | No                                  | Yes                                 | No                               |
/// | Cached resolution         | Parameterless scenes only           | Yes                                 | No                               |
///
/// Prefer scene inheritance over inline composition in general: the expensive scene resolution is cached, saving work during reuse.
/// Inline composition should be reserved for parameterized scenes that vary based on a given input,
/// small scenes that are shared across contexts (like styles),
/// or one-off scenes that do not require reuse.
///
/// /// ## Formatting BSN
///
/// Whitespace, parentheses, and comments have no effect on the generated scene —
/// they exist purely to help you organize and read your code.
///
/// **Whitespace** (spaces, newlines, tabs) separates items on the *same* entity.
/// Use it freely for alignment and to make groupings of both components and entities clearer.
///
/// **Parentheses** group a set of items into one logical unit inside a `[...]` list.
/// Trailing commas are the delimiter used to end one entity and start another,
/// but these can be hard to spot in deeply nested or one-line scenes.
/// Parentheses (often with associated indentation) make the boundary explicit:
///
/// ```rust, ignore
/// bsn! {
///     Children [
///         // Hard to see where one entity ends and the next begins:
///         ComponentA
///         ComponentB,
///         ComponentA
///         ComponentC,
///
///         // Much clearer:
///         (
///           ComponentA
///           ComponentB
///         ),
///         (
///           ComponentA
///           ComponentC
///         ),
///     ]
/// }
/// ```
///
/// ```rust, ignore
/// // Without parentheses, one-line definitions of children are prone to subtle mistakes:
/// bsn! { Children [ComponentA ComponentB, ComponentC ComponentD] }
///
/// // With parentheses, the structure is clear:
/// bsn! { Children [ (ComponentA ComponentB), (ComponentC ComponentD) ] }
/// ```
///
/// **Comments** (`//` line comments and `/* */` block comments) work exactly as in
/// normal Rust and are stripped by the macro before parsing.
///
/// ## Syntax Reference
///
/// ### Components on the same entity vs. separate entities
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `CompA CompB CompC` | Same entity | **Whitespace** between items — all go on the **same** entity |
/// | `A, B, C` | Separate entities | **Commas** inside `[…]` — each becomes its **own** entity |
/// | `(CompA CompB)` | Entity group | Parentheses group components for readability; equivalent to whitespace alone |
///
/// ### Naming entities
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `#Name` | Named entity | Adds a `Name("Name")` component and registers the entity for cross-referencing within this scope |
/// | `#{ expr }` | Dynamic name | Names an entity using the result of a Rust expression |
///
/// ### Relationships
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `Children [s1, s2]` | Add children | Spawns each entry as a child **of this** entity |
/// | `ChildOf(entity)` | Set parent | Makes **this** entity a child of `entity`; accepts a plain `Entity` or a `#Name` reference |
/// | `MyRel [s1, s2]` | Custom relationship | Like `Children`, but uses any `RelationshipTarget` component |
///
/// ### Dynamic values
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `{ expr }` | Rust expression | Evaluated at spawn time; may be a component, `impl Scene`, or (inside `[…]`) `impl SceneList` |
/// | `field: { expr }` | Expression in field | Embeds a Rust expression as the value of a named field |
///
/// ### Composition and inheritance
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `my_scene()` | Inline composition | Merges `my_scene`'s unresolved templates into this entity |
/// | `:my_scene` | Inheritance | Pre-resolves `my_scene` first, then patches on top |
/// | `:"path.bsn"` | Asset inheritance | Inherits from a `.bsn` asset file; requires `queue_spawn_scene` |
///
/// ### Observers
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `on(\|ev: On<Ev>\| { … })` | Observer | Attaches an entity observer that fires when `Ev` targets this entity |
///
/// ### Other Rust syntax
///
/// If you're new to Rust, you might struggle with some of its syntax when you see it in BSN.
/// Here are the most important syntax patterns to be aware of:
///
/// | Syntax | Meaning | Explanation |
/// |--------|---------|-------------|
/// | `MyComponent` | Unit or defaulted struct | A struct with no fields, or in BSN only, with all fields at their defaults |
/// | `MyComponent { field: val, field2: val2 }` | Struct with named fields | Sets named fields; unmentioned fields keep their defaults or values from prior patches |
/// | `MyComponent(val1, val2)` | Tuple struct | Constructs a tuple-struct component |
/// | `MyEnum::Variant` | Enum variant | A value of the `MyEnum` type with the `Variant` variant |
/// | `module::MyComponent` | Path | A module path to a struct type |
/// | `GREEN`` | Constant | A constant value |
/// | `f32::PI` | Associated constant | A constant (here, `PI`) associated with a type (here, `f32`) |
/// | `\|param\| { … }` | Closure | A closure; effectively an unnamed function |
/// | `Vec<T>` | Generic type | A type with a generic type parameter `T` |
/// | `spawn_enemy("orc", 10, true)` | Function call | Calls a function with the given arguments by position |
/// | `spawn_player()` | Argumentless function call | Calls a function that takes no arguments |
/// | `//` | Comment | Line comment; all text after `//` on the same line is ignored |
/// | `/* */` | Block comment | Standard Rust block comment; all text inside `/* */` is ignored |
/// | `bsn! { … }` | Macro call | Calls a macro on the value inside of the braces |
///
/// ## Further reading
///
/// See [`bsn_list!`] if you want to create multiple scenes at once,
/// or want to have multiple root entities.
///
/// See the `bevy_scene` crate docs for a high-level overview of the key concepts.
#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::bsn::bsn(input)
}

/// Creates a `SceneList` using BSN (Bevy Scene Notation) syntax.
///
/// This is useful when you want multiple root entities in your scene
/// that do not share a common parent, or if you want to create multiple scenes at once.
///
/// Like in [`bsn!`], commas separate entities,
/// while whitespace separates components on the same entity.
///
/// All root entries in a [`bsn_list!`] share a single name scope, so sibling root entities
/// can cross-reference each other by `#Name`.
/// This is not possible with separate [`bsn!`] calls, and is a key motivation for using [`bsn_list!`].
///
/// See [`bsn!`] for more details on syntax.
/// See the `bevy_scene` crate docs for a high-level overview of the key concepts.
#[proc_macro]
pub fn bsn_list(input: TokenStream) -> TokenStream {
    crate::bsn::bsn_list(input)
}

#[proc_macro_derive(
    SceneComponent,
    attributes(component, require, relationship, relationship_target, entities, scene)
)]
pub fn derive_scene_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(scene_component::derive_scene_component(&mut ast))
}
