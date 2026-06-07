use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token, Expr, Path, Token};

/// A `TypeData` registration.
///
/// This would be the `Default` and `Hash(custom_hash_fn)` in
/// `#[reflect(Default, Hash(custom_hash_fn))]`.
#[derive(Clone)]
pub(crate) struct TypeDataRegistration {
    path: Path,
    reflect_path: Path,
    args: Punctuated<Expr, Token![,]>,
}

impl TypeDataRegistration {
    /// The original path of the registration.
    ///
    /// If the last ident in the path is already prefixed with `Reflect`,
    /// then this should be the same as the path returned by [`Self::reflect_path`].
    ///
    /// Examples:
    /// - `#[reflect(Foo)]` would give `Foo`
    /// - `#[reflect(ReflectFoo)]` would give `ReflectFoo`
    /// - `#[reflect(crate::foo::Foo)]` would give `crate::foo::Foo`
    /// - `#[reflect(crate::foo::ReflectFoo)]` would give `crate::foo::ReflectFoo`
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The reflected type data path of the registration.
    ///
    /// If the last ident in the path is already prefixed with `Reflect`,
    /// then this should be the same as the path returned by [`Self::path`].
    ///
    /// Examples:
    /// - `#[reflect(Foo)]` would give `ReflectFoo`
    /// - `#[reflect(ReflectFoo)]` would give `ReflectFoo`
    /// - `#[reflect(crate::foo::Foo)]` would give `crate::foo::ReflectFoo`
    /// - `#[reflect(crate::foo::ReflectFoo)]` would give `crate::foo::ReflectFoo`
    pub fn reflect_path(&self) -> &Path {
        &self.reflect_path
    }

    /// The optional arguments of the type data.
    pub fn args(&self) -> &Punctuated<Expr, Token![,]> {
        &self.args
    }
}

impl Parse for TypeDataRegistration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<Path>()?;
        let reflect_path = crate::ident::get_reflect_path(&path);

        let args = if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            content.parse_terminated(Expr::parse, Token![,])?
        } else {
            Default::default()
        };

        Ok(Self {
            path,
            reflect_path,
            args,
        })
    }
}
