#![allow(clippy::type_complexity)]

extern crate proc_macro;

mod app_plugin;
mod bevy_main;
mod derefs;
mod enum_variant_meta;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;

/// Generates a dynamic plugin entry point function for the given `Plugin` type.
#[proc_macro_derive(DynamicPlugin)]
pub fn derive_dynamic_plugin(input: TokenStream) -> TokenStream {
    app_plugin::derive_dynamic_plugin(input)
}

/// Implements [`Deref`] for structs. This is especially useful when utilizing the [newtype] pattern.
///
/// For single-field structs, the implementation automatically uses that field.
/// For multi-field structs, you must specify which field to use with the `#[deref]` attribute.
///
/// If you need [`DerefMut`] as well, consider using the other [derive] macro alongside
/// this one.
///
/// # Example
///
/// ## Tuple Structs
///
/// Using a single-field struct:
///
/// ```
/// use bevy_derive::Deref;
///
/// #[derive(Deref)]
/// struct MyNewtype(String);
///
/// let foo = MyNewtype(String::from("Hello"));
/// assert_eq!("Hello", *foo);
/// ```
///
/// Using a multi-field struct:
///
/// ```
/// # use std::marker::PhantomData;
/// use bevy_derive::Deref;
///
/// #[derive(Deref)]
/// struct MyStruct<T>(#[deref] String, PhantomData<T>);
///
/// let foo = MyStruct(String::from("Hello"), PhantomData::<usize>);
/// assert_eq!("Hello", *foo);
/// ```
///
/// ## Named Structs
///
/// Using a single-field struct:
///
/// ```
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyStruct {
///   value: String,
/// }
///
/// let foo = MyStruct {
///   value: String::from("Hello")
/// };
/// assert_eq!("Hello", *foo);
/// ```
///
/// Using a multi-field struct:
///
/// ```
/// # use std::marker::PhantomData;
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyStruct<T> {
///   #[deref]
///   value: String,
///   _phantom: PhantomData<T>,
/// }
///
/// let foo = MyStruct {
///   value:String::from("Hello"),
///   _phantom:PhantomData::<usize>
/// };
/// assert_eq!("Hello", *foo);
/// ```
///
/// [`Deref`]: std::ops::Deref
/// [newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
/// [`DerefMut`]: std::ops::DerefMut
/// [derive]: crate::derive_deref_mut
#[proc_macro_derive(Deref, attributes(deref))]
pub fn derive_deref(input: TokenStream) -> TokenStream {
    derefs::derive_deref(input)
}

/// Implements [`DerefMut`] for structs. This is especially useful when utilizing the [newtype] pattern.
///
/// For single-field structs, the implementation automatically uses that field.
/// For multi-field structs, you must specify which field to use with the `#[deref]` attribute.
///
/// [`DerefMut`] requires a [`Deref`] implementation. You can implement it manually or use
/// Bevy's [derive] macro for convenience.
///
/// # Example
///
/// ## Tuple Structs
///
/// Using a single-field struct:
///
/// ```
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyNewtype(String);
///
/// let mut foo = MyNewtype(String::from("Hello"));
/// foo.push_str(" World!");
/// assert_eq!("Hello World!", *foo);
/// ```
///
/// Using a multi-field struct:
///
/// ```
/// # use std::marker::PhantomData;
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyStruct<T>(#[deref] String, PhantomData<T>);
///
/// let mut foo = MyStruct(String::from("Hello"), PhantomData::<usize>);
/// foo.push_str(" World!");
/// assert_eq!("Hello World!", *foo);
/// ```
///
/// ## Named Structs
///
/// Using a single-field struct:
///
/// ```
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyStruct {
///   value: String,
/// }
///
/// let mut foo = MyStruct {
///   value: String::from("Hello")
/// };
/// foo.push_str(" World!");
/// assert_eq!("Hello World!", *foo);
/// ```
///
/// Using a multi-field struct:
///
/// ```
/// # use std::marker::PhantomData;
/// use bevy_derive::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyStruct<T> {
///   #[deref]
///   value: String,
///   _phantom: PhantomData<T>,
/// }
///
/// let mut foo = MyStruct {
///   value:String::from("Hello"),
///   _phantom:PhantomData::<usize>
/// };
/// foo.push_str(" World!");
/// assert_eq!("Hello World!", *foo);
/// ```
///
/// [`DerefMut`]: std::ops::DerefMut
/// [newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
/// [`Deref`]: std::ops::Deref
/// [derive]: crate::derive_deref
#[proc_macro_derive(DerefMut, attributes(deref))]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
    derefs::derive_deref_mut(input)
}

#[proc_macro_attribute]
pub fn bevy_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    bevy_main::bevy_main(attr, item)
}

#[proc_macro_derive(EnumVariantMeta)]
pub fn derive_enum_variant_meta(input: TokenStream) -> TokenStream {
    enum_variant_meta::derive_enum_variant_meta(input)
}

/// Generates an impl of the `AppLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[app_label(ignore_fields)]`.
#[proc_macro_derive(AppLabel, attributes(app_label))]
pub fn derive_app_label(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let mut trait_path = BevyManifest::default().get_path("bevy_app");
    trait_path.segments.push(format_ident!("AppLabel").into());
    derive_label(input, &trait_path, "app_label")
}
