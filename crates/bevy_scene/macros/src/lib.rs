mod bsn;
mod scene_component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Macro which returns a [`Scene`](https://docs.rs/bevy/latest/bevy/prelude/trait.Scene.html), comprehensive docs at [`bevy_scene`](https://docs.rs/bevy/latest/bevy/scene/index.html)
///
/// Example macro, intended as a hint for allowed syntax.:
/// ```rust,ignore
/// bsn! {
///     :some_scene // inherit from a scene function, its
///     #SomeName // entity name, will insert Name("SomeName")
///     ComponentA // component without a value will use default
///     ComponentB(0.0) // passing a value, other fields will use default
///     Node {
///         height: px(0.1) // same with named fields, unmentioned ones stay default
///     }
///     on(|evt: On<MyEntityEvent>, mut query: Query<&mut ComponentB>| { // add an observer
///         let mut b = query.get_mut(evt.entity).unwrap(); // unwrap since we're sure this entity always has ComponentA
///         b.0 += evt.value;
///     })
///     Children [ // spawning multiple related entities using a RelationshipTarget component
///         #Child1 ComponentA // whitespace doesn't have to be newlines
///         , // entities are comma-separated
///         (:other_scene #Child3), // parentheses around a single entity optional
///         Link(#SomeName), // passing a entity reference to a component as `Entity`, component has to implement FromTemplate
///         @MySceneComponent {  // components which derive SceneComponent have scenes and can be inherited from
///             @some_prop: 3, // props, look like fields prefixed with @ but end up passed to the components scene as arguments
///             normal_field: 5 // while normal fields are the actual fields of the component
///         },
///         Node {
///         width: some_variable
///         }
///         ComponentB({some_variable + 3.}) // values can be expressions, when wrapped in {}
///         @Container {
///             @items: {
///                 bsn_list![ // sometimes you may need to nest macro calls
///                     #item1 SomeComponent, // note: the name #item1 here is in its own scope
///                     :some_scene #item2
///                 ]
///             }
///         }
///     ]
/// };
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::bsn::bsn(input)
}

///
#[doc(hidden)]
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
