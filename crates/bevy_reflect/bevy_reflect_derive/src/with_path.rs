use std::path;

use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Generics, LitStr, Path, Token, Type, TypeArray, TypeParamBound, TypePath, Expr,
};

use crate::derive_data::ReflectMeta;

pub enum WithPathDef {
    External {
        path: Path,
        generics: Generics,
    },
    AliasedNamed {
        generics: Generics,
        ty: Type,
        alias: Path,
    },
    AliasedAnonymous {
        generics: Generics,
        ty: Type,
        alias: Type,
    },
}

pub fn parse_path_leading_colon(input: ParseStream) -> syn::Result<Path> {
    let leading = input.parse::<Token![::]>()?;

    if input.peek(Token![::]) {
        return Err(input.error("did not expect two leading double colons (`::::`)"));
    }

    let mut path = Path::parse_mod_style(input)?;
    path.leading_colon = Some(leading);
    Ok(path)
}

pub fn parse_path_no_leading_colon(input: ParseStream) -> syn::Result<Path> {
    if input.peek(Token![::]) {
        return Err(input.error("did not expect a leading double colon (`::`)"));
    }

    Path::parse_mod_style(input)
}

impl Parse for WithPathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (generics, ty) = if input.peek(Token![::]) {
            let path = parse_path_leading_colon(input)?;
            let mut generics = input.parse::<Generics>()?;
            generics.where_clause = input.parse()?;

            if !input.peek(Token![as]) {
                return Ok(WithPathDef::External { path, generics });
            }

            (
                Generics::default(),
                Type::Path(TypePath { qself: None, path }),
            )
        } else {
            let generics = input.parse()?;
            let ty = input.parse()?;
            (generics, ty)
        };

        let _as_token: Token![as] = input.parse()?;

        let alias = parse_path_no_leading_colon(input)?;

        Ok(WithPathDef::AliasedNamed {
            generics,
            ty,
            alias,
        })
    }
}

macro_rules! format_lit {
    ($($arg:tt)*) => {{
        LitStr::new(&format!($($arg)*), proc_macro2::Span::call_site()).to_token_stream()
    }}
}

struct StrExprBuilder<'a> {
    meta: &'a ReflectMeta<'a>,
    stream: proc_macro2::TokenStream,
}

impl<'a> StrExprBuilder<'a> {
    pub fn new(meta: &'a ReflectMeta<'a>) -> Self {
        Self {
            meta,
            stream: proc_macro2::TokenStream::new(),
        }
    }
    
    pub fn append(&mut self, tokens: proc_macro2::TokenStream) {
        if self.stream.is_empty() {
            quote!((#tokens).to_owned()).to_tokens(&mut self.stream);
        } else {
            quote!(+ #tokens).to_tokens(&mut self.stream);
        }
    }
    
    pub fn append_owned(&mut self, tokens: proc_macro2::TokenStream) {
        if self.stream.is_empty() {
            quote!(#tokens).to_tokens(&mut self.stream);
        } else {
            quote!(+ &#tokens).to_tokens(&mut self.stream);
        }
    }
}

trait ToAnonymousPath {
    fn to_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()>;
    fn to_short_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()>;
}

impl ToAnonymousPath for Expr {
    fn to_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()> {
        path.append_owned(quote! {
            ::std::string::ToString::to_string(#self)
        });
        Ok(())
    }
    
    fn to_short_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()> {
        self.to_path_str(path)
    }
}

impl ToAnonymousPath for Path {
    fn to_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()> {
        
        path.append_owned(quote! {
            ::std::string::ToString::to_string(#self)
        });
        Ok(())
    }
    
    fn to_short_path_str(&self, path: &mut StrExprBuilder) -> syn::Result<()> {
        self.to_path_str(path)
    }
}

fn type_to_path_str(ty: &Type, meta: &ReflectMeta) -> syn::Result<proc_macro2::TokenStream> {
    fn path_to_path_str(path: &Path) -> syn::Result<proc_macro2::TokenStream> {
        path.segments.
    }
    
    let bevy_reflect_path = meta.bevy_reflect_path();
    
    Ok(match ty {
        Type::Array(array) => format_lit!(
            "[{elem}; {len}]",
            elem = type_to_path_str(&array.elem, meta)?,
            len = array.len
        ),
        Type::BareFn(bare_fn) => type_to_path_str(ty, meta),
        Type::Group(group) => type_to_path_str(&group.elem, meta)?,
        Type::ImplTrait(_) => todo!(),
        Type::Infer(_) => todo!(),
        Type::Macro(_) => todo!(),
        Type::Never(_) => quote!(!),
        Type::Paren(paren) => type_to_path_str(&paren.elem, meta)?,
        Type::Path(path) => quote! {
            <#path as #bevy_reflect_path::WithPath>::type_path().path()
        },
        Type::Ptr(ptr) => format_lit!(
            "*{m} {t}",
            m = ptr.mutability.map(|_| "mut").unwrap_or_else(|| "const"),
            t = type_to_path_str(&ptr.elem, meta)?,
        ),
        Type::Reference(reference) => format_lit!(
            "&{m}{t}",
            m = reference.mutability.map(|_| "mut ").unwrap_or_default(),
            t = type_to_path_str(&reference.elem, meta)?,
        ),
        Type::Slice(slice) => format_lit!("[{}]", type_to_path_str(&slice.elem, meta)?),
        Type::TraitObject(trait_object) => {
            let bounds: syn::Result<proc_macro2::TokenStream> = trait_object.bounds.iter().filter_map(
                    |bound| if let TypeParamBound::Trait(t) = bound {
                        Some(path_to_path_str(&t.path))
                    } else {
                        None
                    }
                ).collect();
            format_lit!("dyn {}", bounds?)
        },
        Type::Tuple(_) => todo!(),
        Type::Verbatim(_) => todo!(),
        _ => todo!(),
    })
}
