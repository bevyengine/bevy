// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use proc_macro::TokenStream;
use proc_macro2::{Span as Span2, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned as _,
    token::Comma,
    Attribute, Error, Ident, LitInt, LitStr, Result,
};
struct AllTuples {
    fake_variadic: bool,
    macro_ident: Ident,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let fake_variadic = input.call(parse_fake_variadic_attr)?;
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

        if start > 1 && fake_variadic {
            return Err(Error::new(
                input.span(),
                "#[doc(fake_variadic)] only works when the tuple with length one is included",
            ));
        }

        Ok(AllTuples {
            fake_variadic,
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
/// // impl_wrapped_in_foo!(T0);
/// // impl_wrapped_in_foo!(T0, T1);
/// // ..
/// // impl_wrapped_in_foo!(T0 .. T14);
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
/// ```
///
/// **`#[doc(fake_variadic)]`**
///
/// To improve the readability of your docs when implementing a trait for
/// tuples or fn pointers of varying length you can use the rustdoc-internal `fake_variadic` marker.
/// All your impls are collapsed and shown as a single `impl Trait for (F₁, F₂, …, Fₙ)`.
///
/// The `all_tuples!` macro does most of the work for you, the only change to your implementation macro
/// is that you have to accept attributes using `$(#[$meta:meta])*`.
///
/// Since this feature requires a nightly compiler, it's only enabled on docs.rs by default.
/// Add the following to your lib.rs if not already present:
///
/// ```
/// // `rustdoc_internals` is needed for `#[doc(fake_variadics)]`
/// #![allow(internal_features)]
/// #![cfg_attr(any(docsrs, docsrs_dep), feature(rustdoc_internals))]
/// ```
///
/// ```
/// use bevy_utils_proc_macros::all_tuples;
///
/// trait Variadic {}
///
/// impl Variadic for () {}
///
/// macro_rules! impl_append {
///     ($(#[$meta:meta])* $(($P:ident, $p:ident)),*) => {
///         $(#[$meta])*
///         impl<$($P),*> Variadic for ($($P,)*) {}
///     }
/// }
///
/// all_tuples!(#[doc(fake_variadic)] impl_variadic, 1, 15, P, p);
/// ```
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
        let ident_tuples = choose_ident_tuples(&input, &ident_tuples, i);
        let attrs = if input.fake_variadic {
            fake_variadic_attrs(len, i)
        } else {
            TokenStream2::default()
        };
        quote! {
            #macro_ident!(#attrs #ident_tuples);
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

/// Parses the attribute `#[doc(fake_variadic)]`
fn parse_fake_variadic_attr(input: ParseStream) -> Result<bool> {
    let attribute = match input.call(Attribute::parse_outer)? {
        attributes if attributes.len() == 0 => return Ok(false),
        attributes if attributes.len() == 1 => attributes[0].clone(),
        attributes => {
            return Err(Error::new(
                input.span(),
                format!("Expected exactly one attribute, got {}", attributes.len()),
            ))
        }
    };

    if attribute.path().is_ident("doc") {
        let nested = attribute.parse_args::<Ident>()?;
        if nested == "fake_variadic" {
            return Ok(true);
        }
    }

    Err(Error::new(
        attribute.meta.span(),
        format!("Unexpected attribute"),
    ))
}

fn to_ident_tuple(idents: impl Iterator<Item = Ident>, len: usize) -> TokenStream2 {
    if len < 2 {
        quote! { #(#idents)* }
    } else {
        quote! { (#(#idents),*) }
    }
}

fn choose_ident_tuples<'a>(
    input: &AllTuples,
    ident_tuples: &'a [TokenStream2],
    i: usize,
) -> TokenStream2 {
    // `rustdoc` uses the first ident to generate nice
    // idents with subscript numbers e.g. (F₁, F₂, …, Fₙ).
    // We don't want two numbers, so we use the
    // original, unnumbered idents for this case.
    if input.fake_variadic && i == 1 {
        let ident_tuple = to_ident_tuple(input.idents.iter().cloned(), input.idents.len());
        quote! { #ident_tuple }
    } else {
        let ident_tuples = &ident_tuples[..i];
        quote! { #(#ident_tuples),* }
    }
}

fn fake_variadic_attrs(len: usize, i: usize) -> TokenStream2 {
    let cfg = quote! { any(docsrs, docsrs_dep) };
    match i {
        // An empty tuple (i.e. the unit type) is still documented separately,
        // so no `#[doc(hidden)]` here.
        0 => TokenStream2::default(),
        // The `#[doc(fake_variadic)]` attr has to be on the first impl block.
        1 => {
            let doc = LitStr::new(
                &format!("This trait is implemented for tuples up to {len} items long."),
                Span2::call_site(),
            );
            quote! {
                #[cfg_attr(#cfg, doc(fake_variadic))]
                #[cfg_attr(#cfg, doc = #doc)]
            }
        }
        _ => quote! { #[cfg_attr(#cfg, doc(hidden))] },
    }
}
