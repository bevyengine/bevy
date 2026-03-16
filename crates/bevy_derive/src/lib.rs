//! Assorted proc macro derive functions.

#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

extern crate proc_macro;

mod bevy_main;
mod derefs;
mod enum_variant_meta;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;

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

/// Generates the required main function boilerplate for Android.
#[proc_macro_attribute]
pub fn bevy_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    bevy_main::bevy_main(attr, item)
}

/// Adds `enum_variant_index` and `enum_variant_name` functions to enums.
///
/// # Example
///
/// ```
/// use bevy_derive::{EnumVariantMeta};
///
/// #[derive(EnumVariantMeta)]
/// enum MyEnum {
///     A,
///     B,
/// }
///
/// let a = MyEnum::A;
/// let b = MyEnum::B;
///
/// assert_eq!(0, a.enum_variant_index());
/// assert_eq!("A", a.enum_variant_name());
///
/// assert_eq!(1, b.enum_variant_index());
/// assert_eq!("B", b.enum_variant_name());
/// ```
#[proc_macro_derive(EnumVariantMeta)]
pub fn derive_enum_variant_meta(input: TokenStream) -> TokenStream {
    enum_variant_meta::derive_enum_variant_meta(input)
}

/// Generates an impl of the `AppLabelBase` trait.
///
/// This does not work for unions.
#[proc_macro_derive(AppLabelBase)]
pub fn derive_app_label_base(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let mut trait_path = BevyManifest::shared(|manifest| manifest.get_path("bevy_app"));
    trait_path
        .segments
        .push(format_ident!("AppLabelBase").into());
    derive_label(input, "AppLabelBase", &trait_path)
}

extern crate quote;
use quote::quote;
// extern crate syn;

/// AppLabel helper
#[proc_macro_attribute]
pub fn app_label(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.clone().into();

    let input1 = syn::parse_macro_input!(input as syn::DeriveInput);
    let ident = input1.ident.clone();
    let (impl_generics, ty_generics, where_clause) = input1.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause.predicates.push(
        syn::parse2(quote! {
            Self: 'static + Send + Sync + Clone + Default + Copy + Eq + ::core::fmt::Debug + ::core::hash::Hash
        })
        .unwrap(),
    );

    let bevy_app_path = BevyManifest::shared(|manifest| manifest.get_path("bevy_app"));

    let mut app_label_base_path = bevy_app_path.clone();
    app_label_base_path
        .segments
        .push(format_ident!("AppLabelBase").into());

    let mut app_label_path = bevy_app_path.clone();
    app_label_path
        .segments
        .push(format_ident!("AppLabel").into());

    let output = quote! {
        #[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, #app_label_base_path)]
        #input2

        impl #impl_generics #app_label_path for #ident #ty_generics #where_clause {
            // fn intern(&self) {
            //     (self as #app_label_base_path).intern(self)
            // }
        }
    };
    output.into()
}
