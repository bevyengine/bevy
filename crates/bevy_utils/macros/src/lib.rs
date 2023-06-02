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
/// ```
/// use bevy_utils_proc_macros::all_tuples;
///
/// trait Foo {
///     // ..
/// }
///
/// macro_rules! impl_foo {
///     ($($P:ident),*) => {
///         // ..
///     }
/// }
///
/// all_tuples!(impl_foo, 0, 16, P);
/// // impl_foo!();
/// // impl_foo!(P0);
/// // impl_foo!(P0, P1);
/// // ..
/// // impl_foo!(P0 .. P15);
/// ```
/// ```
/// use bevy_utils_proc_macros::all_tuples;
///
/// trait Foo {
///     // ..
/// }
///
/// macro_rules! impl_foo {
///     ($(($P:ident, $Q:ident)),*) => {
///         // ..
///     }
/// }
///
/// all_tuples!(impl_foo, 2, 16, P, Q);
/// // impl_foo!((P0, Q0), (P1, Q1));
/// // impl_foo!((P0, Q0), (P1, Q1), (P2, Q2));
/// // impl_foo!((P0, Q0), (P1, Q1), (P2, Q2), (P3, Q3));
/// // ..
/// // impl_foo!((P0, Q0) .. (P15, Q15));
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
