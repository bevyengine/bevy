use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, LitStr};

/// Contains tokens representing different kinds of string.
#[derive(Clone)]
pub(crate) enum StringExpression {
    /// A string that is valid at compile time.
    ///
    /// This is either a string literal like `"mystring"`,
    /// or a string created by a macro like [`module_path`]
    /// or [`concat`].
    Const(TokenStream),
    /// A [string slice](str) that is borrowed for a `'static` lifetime.
    Borrowed(TokenStream),
    /// An [owned string](String).
    Owned(TokenStream),
}

impl<T: ToString + Spanned> From<T> for StringExpression {
    fn from(value: T) -> Self {
        Self::from_lit(&LitStr::new(&value.to_string(), value.span()))
    }
}

impl StringExpression {
    /// Creates a [constant] [`StringExpression`] from a [`struct@LitStr`].
    ///
    /// [constant]: StringExpression::Const
    pub fn from_lit(lit: &LitStr) -> Self {
        Self::Const(lit.to_token_stream())
    }

    /// Creates a [constant] [`StringExpression`] by interpreting a [string slice][str] as a [`struct@LitStr`].
    ///
    /// [constant]: StringExpression::Const
    pub fn from_str(string: &str) -> Self {
        Self::Const(string.into_token_stream())
    }

    /// Returns tokens for an [owned string](String).
    ///
    /// The returned expression will allocate unless the [`StringExpression`] is [already owned].
    ///
    /// [already owned]: StringExpression::Owned
    pub fn into_owned(self) -> TokenStream {
        match self {
            Self::Const(tokens) | Self::Borrowed(tokens) => quote! {
                ::std::string::ToString::to_string(#tokens)
            },
            Self::Owned(owned) => owned,
        }
    }

    /// Returns tokens for a statically borrowed [string slice](str).
    pub fn into_borrowed(self) -> TokenStream {
        match self {
            Self::Const(tokens) | Self::Borrowed(tokens) => tokens,
            Self::Owned(owned) => quote! {
                &#owned
            },
        }
    }

    /// Appends a [`StringExpression`] to another.
    ///
    /// If both expressions are [`StringExpression::Const`] this will use [`concat`] to merge them.
    pub fn appended_by(mut self, other: StringExpression) -> Self {
        if let Self::Const(tokens) = self {
            if let Self::Const(more) = other {
                return Self::Const(quote! {
                    ::core::concat!(#tokens, #more)
                });
            }
            self = Self::Const(tokens);
        }

        let owned = self.into_owned();
        let borrowed = other.into_borrowed();
        Self::Owned(quote! {
            #owned + #borrowed
        })
    }
}

impl Default for StringExpression {
    fn default() -> Self {
        StringExpression::from_str("")
    }
}

impl FromIterator<StringExpression> for StringExpression {
    fn from_iter<T: IntoIterator<Item = StringExpression>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        match iter.next() {
            Some(mut expr) => {
                for next in iter {
                    expr = expr.appended_by(next);
                }

                expr
            }
            None => Default::default(),
        }
    }
}
