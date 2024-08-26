use proc_macro::{TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use std::collections::HashSet;
use syn::{spanned::Spanned, Ident};

/// Finds an identifier that will not conflict with the specified set of tokens.
/// If the identifier is present in `haystack`, extra characters will be added
/// to it until it no longer conflicts with anything.
///
/// Note that the returned identifier can still conflict in niche cases,
/// such as if an identifier in `haystack` is hidden behind an un-expanded macro.
pub fn ensure_no_collision(value: Ident, haystack: TokenStream) -> Ident {
    // Collect all the identifiers in `haystack` into a set.
    let idents = {
        // List of token streams that will be visited in future loop iterations.
        let mut unvisited = vec![haystack];
        // Identifiers we have found while searching tokens.
        let mut found = HashSet::new();
        while let Some(tokens) = unvisited.pop() {
            for t in tokens {
                match t {
                    // Collect any identifiers we encounter.
                    TokenTree::Ident(ident) => {
                        found.insert(ident.to_string());
                    }
                    // Queue up nested token streams to be visited in a future loop iteration.
                    TokenTree::Group(g) => unvisited.push(g.stream()),
                    TokenTree::Punct(_) | TokenTree::Literal(_) => {}
                }
            }
        }

        found
    };

    let span = value.span();

    // If there's a collision, add more characters to the identifier
    // until it doesn't collide with anything anymore.
    let mut value = value.to_string();
    while idents.contains(&value) {
        value.push('X');
    }

    Ident::new(&value, span)
}

/// Derive a label trait
///
/// # Args
///
/// - `input`: The [`syn::DeriveInput`] for struct that is deriving the label trait
/// - `trait_name`: Name of the label trait
/// - `trait_path`: The [path](`syn::Path`) to the label trait
/// - `dyn_eq_path`: The [path](`syn::Path`) to the `DynEq` trait
pub fn derive_label(
    input: syn::DeriveInput,
    trait_name: &str,
    trait_path: &syn::Path,
    dyn_eq_path: &syn::Path,
) -> TokenStream {
    if let syn::Data::Union(_) = &input.data {
        let message = format!("Cannot derive {trait_name} for unions.");
        return quote_spanned! {
            input.span() => compile_error!(#message);
        }
        .into();
    }

    let ident = input.ident.clone();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    where_clause.predicates.push(
        syn::parse2(quote! {
            Self: 'static + Send + Sync + Clone + Eq + ::std::fmt::Debug + ::std::hash::Hash
        })
        .unwrap(),
    );
    quote! {
        impl #impl_generics #trait_path for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> ::std::boxed::Box<dyn #trait_path> {
                ::std::boxed::Box::new(::std::clone::Clone::clone(self))
            }

            fn as_dyn_eq(&self) -> &dyn #dyn_eq_path {
                self
            }

            fn dyn_hash(&self, mut state: &mut dyn ::std::hash::Hasher) {
                let ty_id = ::std::any::TypeId::of::<Self>();
                ::std::hash::Hash::hash(&ty_id, &mut state);
                ::std::hash::Hash::hash(self, &mut state);
            }
        }
    }
    .into()
}
