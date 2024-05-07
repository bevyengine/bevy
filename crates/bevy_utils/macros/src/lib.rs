// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    Ident, LitInt, Result,
};
struct AllTuples {
    macro_ident: Ident,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let mut idents = vec![input.parse::<Ident>()?];
        while input.parse::<Comma>().is_ok() {
            idents.push(input.parse::<Ident>()?);
        }

        Ok(AllTuples {
            macro_ident,
            start,
            end,
            idents,
        })
    }
}

/// Helper macro to generate tuple pyramids. Useful to generate scaffolding to work around Rust
/// lacking variadics. Invoking `all_tuples!(impl_foo, start, end, P, Q, ..)`
/// invokes `impl_foo` providing ident tuples through arity `start..=end`.
/// # Examples
/// A single parameter.
/// ```
/// use std::marker::PhantomData;
/// use bevy_utils_proc_macros::all_tuples;
///
/// struct Foo<T> {
///     // ..
///     _phantom: PhantomData<T>
/// }
///
/// trait WrappedInFoo {
///     type Tup;
/// }
///
/// macro_rules! impl_wrapped_in_foo {
///     ($($T:ident),*) => {
///         impl<$($T),*> WrappedInFoo for ($($T,)*) {
///             type Tup = ($(Foo<$T>,)*);
///         }
///     };
/// }
///
/// all_tuples!(impl_wrapped_in_foo, 0, 15, T);
/// // impl_wrapped_in_foo!();
/// // impl_wrapped_in_foo!(P0);
/// // impl_wrapped_in_foo!(P0, P1);
/// // ..
/// // impl_wrapped_in_foo!(P0 .. P14);
/// ```
/// Multiple parameters.
/// ```
/// use bevy_utils_proc_macros::all_tuples;
///
/// trait Append {
///     type Out<Item>;
///     fn append<Item>(tup: Self, item: Item) -> Self::Out<Item>;
/// }
///
/// impl Append for () {
///     type Out<Item> = (Item,);
///     fn append<Item>(_: Self, item: Item) -> Self::Out<Item> {
///         (item,)
///     }
/// }
///
/// macro_rules! impl_append {
///     ($(($P:ident, $p:ident)),*) => {
///         impl<$($P),*> Append for ($($P,)*) {
///             type Out<Item> = ($($P),*, Item);
///             fn append<Item>(($($p,)*): Self, item: Item) -> Self::Out<Item> {
///                 ($($p),*, item)
///             }
///         }
///     }
/// }
///
/// all_tuples!(impl_append, 1, 15, P, p);
/// // impl_append!((P0, p0));
/// // impl_append!((P0, p0), (P1, p1));
/// // impl_append!((P0, p0), (P1, p1), (P2, p2));
/// // ..
/// // impl_append!((P0, p0) .. (P14, p14));
/// ````
#[proc_macro]
pub fn all_tuples(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = 1 + input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in 0..=len {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

#[proc_macro]
pub fn all_tuples_with_size(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = 1 + input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in 0..=len {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#i, #(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}
