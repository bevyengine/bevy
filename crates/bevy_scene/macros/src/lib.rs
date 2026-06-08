mod bsn;
mod scene_component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Macro which returns a [`Scene`](https://docs.rs/bevy/latest/bevy/prelude/trait.Scene.html), comprehensive docs at [`bevy_scene`](https://docs.rs/bevy/latest/bevy/scene/index.html)
///
/// Example macro showcasing most syntax (complex, most scenes won't look like this):
/// ```rust,ignore
/// bsn! {
///     some_scene()        // include a scene function
///     #SomeName           // entity name, will insert Name("SomeName")
///     ComponentA          // component without fields: will use the default field values
///     ComponentB(0.0)     // when setting a field, unmentioned fields will use defaults
///     Node {
///         height: px(0.1) // same with named fields, unmentioned ones stay default
///     }
///     on(|evt: On<MyEntityEvent>, mut query: Query<&mut ComponentB>| {  // add an observer
///         let mut b = query.get_mut(evt.entity).unwrap();
///         b.0 += evt.value;
///     })
///     Children [                   // spawning multiple related entities using a RelationshipTarget component
///         #Child1 ComponentA,      // entities are comma-separated
///         (other_scene() #Child3), // parentheses around a single entity are optional for clarity
///         Link(#SomeName),         // passing a entity reference to a component as `Entity`, component has to implement FromTemplate
///         @MySceneComponent {      // components which derive SceneComponent have scenes and can be inherited from
///             @some_prop: 3,       // props, look like fields prefixed with @ but end up passed to the components scene as arguments
///             normal_field: 5      // while normal fields are the actual fields of the component
///         },
///         Node {
///             width: some_var      // variables can be assigned to field values
///         }
///         ComponentB({some_variable + 3.})  // values can be expressions, when wrapped in {}
///         @Container {
///             @items: {
///                 bsn_list![                // sometimes you may need to nest macro calls
///                     #Item1 SomeComponent, // note: the name #Item1 here is in its own scope
///                     some_scene() #Item2
///                 ]
///             }
///         }
///     ]
/// }
/// ```
#[doc(hidden)]
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
/// See [`bsn!`] for an example of the syntax.
/// See the `bevy_scene` crate docs for a high-level overview of the key concepts.#[doc(hidden)]
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
