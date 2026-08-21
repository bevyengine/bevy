// work around Rust-Analyzer issue where it prefers the module highlighting and docs over the macro, even for private modules, if both share a name
// https://github.com/rust-lang/rust-analyzer/issues/19421
// done this way to avoid the large diff of renaming the folder
#[path = "bsn/mod.rs"]
mod _bsn;
mod scene_component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Creates a [`Scene`] using BSN (Bevy Scene Notation) syntax. These docs primarily show syntax.
///
/// See [`bevy_scene`] module-level docs for in-depth details about usage and behavior.
///
/// ## Syntax Reference
///
/// The syntax consists of scene entries which are listed below, which all act on the scene in some way.
/// Often, this is by inserting/patching a component or its values or including other scenes.
/// Scene entries can have prefix characters which specify/disambiguate the following entry.
///
/// ```rust,ignore
/// bsn! {
///     <scene entry>
///     :<cached scene include>
///     #<name>
///     @<SceneComponent>
///     ~<custom Template>
/// }
/// ```
///
/// ### Scene entries
/// | Examples                                   | Explanation                                                                                                    |
/// | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
/// | `CompA`                                    | A unit or default component. Fields, if any exist, will be default                                             |
/// | `CompA(val)`<br>`CompA(val, val)`          | Tuple Component with some fields specified. Unspecified fields will be default, see [patching]                 |
/// | `CompA { name: val }`                      | Component with some fields specified. Unspecified fields will be default, see [patching]                       |
/// | `mymodule::CompA { name: val }`            | Same as above, but referring to the component by module path                                                   |
/// | `CompA { name }`                           | Component with Rust's "field assignment shorthand". Evaluates to `CompA { name: name.into() }`                 |
/// | `MyEnum::Variant`                          | Enum Component `MyEnum` with the `Variant` variant                                                             |
/// | `template_value(component)`                | Insert the component value from a variable `component`                                                         |
/// | `template_value(CompA::from_str("foo"))`   | Insert the component value by immediately calling the constructor                                              |
/// | `template(\|context\| { ... })`              | Register a function/closure returning a Template (eg. Component). Its passed [`context`] allowing World access |
/// | `~MyType`<br>`~MyType {name: var}`         | Type implementing [`Template`], the prefix is used to distinguish it from Components which use [`FromTemplate`]|
/// | **Including Scenes**                       |                                                                                                                |
/// | `scene()`<br>`scene(val)`                  | Include the result of a `impl `[`Scene`] function                                                              |
/// | `{ expr }`                                 | Include the result of `expr`, which should be a [`Scene`]                                                      |
/// | `@MySceneComp`                             | Include a [`SceneComponent`]. Fields, if any exist, will be default                                            |
/// | `@MySceneComp { @prop: val }`              | Include a [`SceneComponent`] with a `prop` field, passed to this components scene function                     |
/// | `@MySceneComp { name: val }`               | Include a [`SceneComponent`] with a normal field, works the same as it does for normal components              |
/// | `@MySceneComp { @prop: val1, name: val2 }` | Include a [`SceneComponent`] with both a `prop` and a field                                                    |
/// | `:"scene.bsn"`                             | <div class="warning">Asset format not yet implemented!</div> Include a cached scene asset file                 |
/// | `:scene()`<br>`:@MySceneComp`              | <div class="warning">Caching for scene includes not yet implemented!</div> Include a cached scene function     |
/// | **Named entity references**                |                                                                                                                |
/// | `#MyName`                                  | Becomes `Name("MyName")` when used as a `part` of a scene                                                      |
/// | `CompA(#MyName)`<br>`scene(#MyName)`       | Referring to the entity which was named `MyName` in this scope, results in an [`EntityTemplate`] being passed  |
/// | `Name("Foo")`                              | Manually sets the Name component, can be put after a `#MyName` to use a custom name while allowing references  |
/// | **Observers**                              |                                                                                                                |
/// | `on(\|ev: On<Ev>\| { … })`                 | Attaches an entity [`observer`] for the [`EntityEvent`] `Ev` to this entity. In this example, using a closure  |
/// | `on(my_observer)`                          | Attaches an entity [`observer`] for the [`EntityEvent`] `Ev` to this entity. In this example, using a function |
/// | **Relationships**                          |                                                                                                                |
/// | `Children []`                              | Spawns each entry as a child of this entity, see **Scene Lists** below for details                             |
/// | `ChildOf(entity)`                          | Makes **this** entity a child of `entity`, accepts an [`Entity`] or a `#Name` reference ([`EntityTemplate`])   |
/// | `MyRel []`                                 | Like [`Children`], but uses any [`RelationshipTarget`] component                                               |
///
/// ### Scene Lists
///
/// Scene list syntax appears in Relationships and the [`bsn_list!`] macro, surrounded by `[ ]`.
///
/// Unlike parts of a scene, which are whitespace-separated, the scenes in a scene list are comma-separated.
/// Each comma-separated single-entity scene uses the same syntax as a single [`bsn!`] macro call.
///
/// Note: The examples below omit the relationship or macro call, `Children [<scene list>]` or `bsn_list![<scene list>]`
///
/// | Example                      | Meaning                                                                                                               |
/// | ---------------------------- | --------------------------------------------------------------------------------------------------------------------- |
/// | `[ #Child1 CompA, #Child2 ]`     | Spawns 2 children, one with `(Name("Child1"), CompA::default())` and the other with `Name("Child2")`              |
/// | `[ (#Child1 CompA), (#Child2) ]` | Same as above, with explicit parentheses                                                                          |
/// | `[ #First, { expr }, #Last ]`   | Spawns an entity with name `First`, then every entity from the [`SceneList`] returned by expr, then one named `Last` |
/// | `[ #First, ({ expr }), #Last ]` | Same as above, but the `expr` should result in a [`Scene`] and will only spawn one entity using it                   |
///
/// ### Values
///
/// Values in BSN (as in `val`,`val1` etc used above) are generally any literal Rust values, plus a few bsn-specific quirks.
///
///
/// | Example                          | Meaning       | Explanation                                                                             |
/// | -------------------------------- | ------------- | --------------------------------------------------------------------------------------- |
/// | `1`                              | Unsigned int  | Positive number, common types: [`usize`], [`u8`], [`u32`], [`u64`]                      |
/// | `1` or `-1`                      | Signed int    | Positive or negative number, common types: [`i32`], [`i64`]                              |
/// | `1.1` or `-0.1` or `1.` or `-2.` | Float         | Floating point number, common types: [`f32`], [`f64`]                                   |
/// | `true` or `false`                | Bool          | Boolean, type: [`bool`]                                                                 |
/// | `"somename"`                     | String        | Text, types: [`String`] or [`&'static str`](str)                                        |
/// | `"mypicture.png"`                | Asset path    | Asset, when used in a field which expects a [`Handle`] to the matching [`Asset`] type     |
/// | `some_function(1)`               | Function call | Calls a function with the provided arguments                                            |
/// | `GREEN`                          | Constant      | Fixed value, must be in scope                                                           |
/// | `std::f32::consts::PI`           | Constant      | Fixed value, uses full path so doesn't need to be in scope                              |
/// | **Expression syntax**            |               |                                                                                         |
/// | `{ 1 + 2 }`                      | Expression    | Any rust expression works in `{}`, in this case addition of 2 integers                  |
/// | `{ vec![true, false] }`          | Vector        | An expression returning a [`Vec`], a collection of multiple items of one specific type. |
/// | `{ bsn!{ Text("foo") Style } }`  | Scene         | Sometimes, you may need to pass a small [`Scene`] as a value to something else            |
///
/// ### Other Rust syntax
///
/// If you're new to Rust, you might struggle with some of its syntax when you see it in or around BSN.
/// Here are some syntax snippets which haven't been shown so far:
///
/// | Syntax            | Meaning       | Explanation                                                     |
/// | ----------------- | ------------- | --------------------------------------------------------------- |
/// | `\|param\| { … }` | Closure       | A closure; effectively an unnamed function                      |
/// | `Vec<T>`          | Generic type  | A type with a generic type parameter `T`                        |
/// | `//`              | Comment       | Line comment; all text after `//` on the same line is ignored   |
/// | `/* */`           | Block comment | Standard Rust block comment; all text inside `/* */` is ignored |
/// | `bsn! { … }`      | Macro call    | Calls a macro on the value inside of the braces                 |
///
/// ### Syntax example
///
/// Example macro showcasing most syntax (complex, most scenes won't look like this):
/// ```rust,ignore
/// bsn! {
///     some_scene()        // include a scene function
///     #SomeName           // entity name, will insert Name("SomeName")
///     ComponentA          // component without a value will use default
///     ComponentB(0.0)     // passing a value, other fields will use default
///     Node {
///         height: px(0.1) // same with named fields, unmentioned ones stay default
///     }
///     on(|evt: On<MyEntityEvent>, mut query: Query<&mut ComponentB>| {  // add an observer
///         let mut b = query.get_mut(evt.entity).unwrap();
///         b.0 += evt.value;
///     })
///     Children [                   // spawning multiple related entities using a RelationshipTarget component
///         #Child1 ComponentA       // whitespace doesn't have to be newlines
///         ,                        // entities are comma-separated
///         (other_scene() #Child3), // parentheses around a single entity are optional
///         Link(#SomeName),         // passing a entity reference to a component as `Entity`, component has to implement FromTemplate
///         @MySceneComponent {      // components which derive SceneComponent have scenes and can be inherited from
///             @some_prop: 3,       // props, look like fields prefixed with @ but end up passed to the components scene as arguments
///             normal_field: 5      // while normal fields are the actual fields of the component
///         },
///         (
///             Node {
///                 width: some_var      // you can directly use variables without {}
///             }
///             ComponentB({some_var + 3.})  // values can be expressions, when wrapped in {}
///             @Container {
///                 @items: {
///                     bsn_list![                // sometimes you may need to nest macro calls
///                         #item1 SomeComponent, // note: the name #item1 here is in its own scope
///                         some_scene() #item2
///                     ]
///                 }
///             }
///         ),
///     ]
/// };
/// ```
///
/// [`Scene`]: https://docs.rs/bevy/latest/bevy/prelude/trait.Scene.html
/// [`context`]: https://docs.rs/bevy/latest/bevy/ecs/template/struct.TemplateContext.html
/// [`EntityTemplate`]: https://docs.rs/bevy/latest/bevy/ecs/template/enum.EntityTemplate.html
/// [`observer`]: https://docs.rs/bevy/latest/bevy/prelude/struct.Observer.html
/// [`EntityEvent`]: https://docs.rs/bevy/latest/bevy/ecs/event/trait.EntityEvent.html
/// [`Entity`]: https://docs.rs/bevy/latest/bevy/ecs/entity/struct.Entity.html
/// [`Children`]: https://docs.rs/bevy/latest/bevy/ecs/hierarchy/struct.Children.html
/// [`SceneComponent`]: https://docs.rs/bevy/latest/bevy/prelude/trait.SceneComponent.html
/// [patching]: https://docs.rs/bevy/latest/bevy/scene/index.html#patching
/// [`bevy_scene`]: https://docs.rs/bevy/latest/bevy/scene/index.html
/// [`Template`]: https://docs.rs/bevy/latest/bevy/ecs/prelude/trait.Template.html
/// [`FromTemplate`]: https://docs.rs/bevy/latest/bevy/ecs/prelude/trait.FromTemplate.html
/// [`RelationshipTarget`]: https://docs.rs/bevy/latest/bevy/ecs/prelude/trait.RelationshipTarget.html
/// [`Handle`]: https://docs.rs/bevy/latest/bevy/asset/enum.Handle.html
/// [`Asset`]: https://docs.rs/bevy/latest/bevy/asset/trait.Asset.html
/// [`SceneList`]: https://docs.rs/bevy/latest/bevy/prelude/trait.SceneList.html
/// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
/// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
///
#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::_bsn::bsn(input)
}

/// Creates a [`SceneList`] using BSN (Bevy Scene Notation) syntax.
///
/// This is useful when you want multiple root entities in your scene
/// that do not share a common parent, or if you want to create multiple scenes at once.
///
/// Like in relationships in [`bsn!`], commas separate entities,
/// while whitespace separates components on the same entity.
///
/// All root entries in a [`bsn_list!`] share a single name scope, so sibling root entities
/// can cross-reference each other by `#Name`.
/// This is not possible with separate [`bsn!`] calls, and is a key motivation for using [`bsn_list!`].
///
/// See [`bsn!`] for an example of the syntax.
/// See the [`bevy_scene`] crate docs for a high-level overview of the key concepts.
///
/// [`SceneList`]: https://docs.rs/bevy/latest/bevy/prelude/trait.SceneList.html
/// [`bevy_scene`]: https://docs.rs/bevy/latest/bevy/scene/index.html
#[proc_macro]
pub fn bsn_list(input: TokenStream) -> TokenStream {
    crate::_bsn::bsn_list(input)
}

#[proc_macro_derive(
    SceneComponent,
    attributes(component, require, relationship, relationship_target, entities, scene)
)]
pub fn derive_scene_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(scene_component::derive_scene_component(&mut ast))
}
