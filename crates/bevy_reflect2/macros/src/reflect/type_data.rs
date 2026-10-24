use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token, Attribute, Expr, Ident, Path, Token};

/// A `TypeData` registration.
///
/// This would be the `Default` and `Hash(custom_hash_fn)` in
/// `#[reflect(Default, Hash(custom_hash_fn))]`.
#[derive(Clone)]
pub(crate) struct TypeDataRegistration {
    path: Path,
    reflect_path: Path,
    args: Punctuated<Expr, Token![,]>,
    /// The `#[cfg(...)]` attributes that conditionally enable the registration (ex:
    /// `#[cfg(feature = "std")] Default`).
    cfg_attrs: Vec<Attribute>,
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

    /// The `#[cfg(...)]` attributes that conditionally enable the registration.
    pub fn cfg_attrs(&self) -> &[Attribute] {
        &self.cfg_attrs
    }

    pub fn set_cfg_attrs(&mut self, cfg_attrs: Vec<Attribute>) {
        self.cfg_attrs = cfg_attrs;
    }
}

impl Parse for TypeDataRegistration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<Path>()?;
        let reflect_path = get_reflect_path(&path);

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
            cfg_attrs: Vec::new(),
        })
    }
}

/// Returns the "reflected" ident for a given string.
///
/// # Example
///
/// ```
/// # use proc_macro2::{Ident, Span};
/// # // We can't import this method because of its visibility.
/// # fn get_reflect_ident(base_ident: &Ident) -> Ident {
/// #     let reflected = format!("Reflect{base_ident}");
/// #     Ident::new(&reflected, base_ident.span())
/// # }
/// let reflected: Ident = get_reflect_ident(&Ident::new("Hash", Span::call_site()));
/// assert_eq!("ReflectHash", reflected.to_string());
/// ```
pub(crate) fn get_reflect_ident(base_ident: &Ident) -> Ident {
    let reflected = format!("Reflect{base_ident}");
    Ident::new(&reflected, base_ident.span())
}

pub(crate) fn get_reflect_path(path: &Path) -> Path {
    let mut path = path.clone();

    match path.segments.last_mut() {
        Some(segment) if !is_reflected_ident(&segment.ident) => {
            segment.ident = get_reflect_ident(&segment.ident);
        }
        _ => {}
    }
    path
}

fn is_reflected_ident(ident: &Ident) -> bool {
    let ident_str = ident.to_string();
    ident_str.starts_with("Reflect")
}
