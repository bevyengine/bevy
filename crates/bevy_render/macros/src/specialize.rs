use bevy_macro_utils::fq_std::FQDefault;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream}, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DataStruct, DeriveInput, Expr, Ident, Index, Member, Meta, MetaList, Path, Stmt, Token, Type
};

const SPECIALIZE_ATTR_IDENT: &str = "specialize";
const SPECIALIZE_ALL_IDENT: &str = "all";

const KEY_ATTR_IDENT: &str = "key";
const KEY_DEFAULT_IDENT: &str = "default";

pub enum SpecializeImplTargets {
    All,
    Specific(Vec<Path>),
}

impl Parse for SpecializeImplTargets {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(ident) = input.parse::<Ident>() {
            if ident == SPECIALIZE_ALL_IDENT {
                return Ok(SpecializeImplTargets::All)
            }
        } 
        input.parse::<Punctuated<Path, Token![,]>>().map(|punctuated| Self::Specific(punctuated.into_iter().collect()))
    }
}

enum Key {
    Whole,
    Default(Span),
    Index(Index),
    Custom(Expr, Span),
}

impl Key {
    pub fn expr(&self) -> Expr {
        match self {
            Key::Whole => parse_quote!(key),
            Key::Default(_) => parse_quote!(#FQDefault::default()),
            Key::Index(index) => {
                let member = Member::Unnamed(index.clone());
                parse_quote!(key.#member)
            }
            Key::Custom(expr, _) => expr.clone(),
        }
    }
}

struct FieldInfo {
    ty: Type,
    member: Member,
    key: Key,
}

impl FieldInfo {
    pub fn key_ty(&self, specialize_path: &Path, target_path: &Path) -> Option<Type> {
        matches!(self.key, Key::Whole | Key::Index(_)).then_some(
            parse_quote!(<#{self.ty} as #specialize_path::Specialize<#target_path>>::Key),
        )
    }

    pub fn specialize_stmt(&self, specialize_path: &Path, target_path: &Path) -> Stmt {
        parse_quote!(<#{self.ty} as #specialize_path::Specialize<#target_path>>::specialize(&self.#{self.member}, #{self.key.expr()}, descriptor);)
    }
}

pub fn impl_specialize(input: TokenStream) -> TokenStream {
    let bevy_render_path: Path = crate::bevy_render_path();
    let specialize_path = {
        let mut path = bevy_render_path.clone();
        path.segments.push(format_ident!("render_resource").into());
        path
    };

    let ast = parse_macro_input!(input as DeriveInput);
    let specialize_attr = ast.attrs.iter().find_map(|attr| {
        if attr.path().is_ident(SPECIALIZE_ATTR_IDENT) {
            if let Meta::List(meta_list) = &attr.meta {
                return Some(meta_list);
            }
        }
        None
    });
    let Some(specialize_meta_list) = specialize_attr else {
        return syn::Error::new(
            Span::call_site(), 
            "#[derive(Specialize) must be accompanied by #[specialize(..Targets)].\n Example usages: #[specialize(RenderPipeline)], #[specialize(all)]"
        ).into_compile_error().into();
    };
    let specialize_attr_tokens = specialize_meta_list.tokens.clone().into();
    let targets = parse_macro_input!(specialize_attr_tokens as SpecializeImplTargets);


    let Data::Struct(DataStruct { fields, .. }) = &ast.data else {
        let error_span = match &ast.data {
            Data::Struct(_) => unreachable!(),
            Data::Enum(data_enum) => data_enum.enum_token.span(),
            Data::Union(data_union) => data_union.union_token.span(),
        };
        return syn::Error::new(error_span, "#[derive(Specialize)]` only supports structs")
            .into_compile_error()
            .into();
    };

    let mut field_info: Vec<FieldInfo> = Vec::new();
    let mut used_count = 0;
    let mut single_index = 0;

    for (index, field) in fields.iter().enumerate() {
        let field_ty = field.ty.clone();
        let field_member = field.ident.clone().map_or(
            Member::Unnamed(Index {
                index: index as u32,
                span: field.span(),
            }),
            Member::Named,
        );
        let key_index = Index {
            index: used_count,
            span: field.span(),
        };

        let mut use_key_field = true;
        let mut key = Key::Index(key_index);
        for attr in &field.attrs {
            if let Meta::List(MetaList { path, tokens, .. }) = &attr.meta {
                if path.is_ident(&KEY_ATTR_IDENT) {
                    let owned_tokens = tokens.clone().into();
                    //TODO: handle #[key(default)]
                    key = Key::Custom(parse_macro_input!(owned_tokens as Expr), attr.span());
                    use_key_field = false;
                }
            }
        }

        if use_key_field {
            used_count += 1;
            single_index = index;
        }

        field_info.push(FieldInfo {
            ty: field_ty,
            member: field_member,
            key,
        });
    }

    if used_count == 1 {
        field_info[single_index].key = Key::Whole;
    }

    match targets {
        SpecializeImplTargets::All => impl_specialize_all(&specialize_path, &ast, &field_info),
        SpecializeImplTargets::Specific(targets) => targets
            .iter()
            .map(|target| impl_specialize_specific(&specialize_path, &ast, &field_info, target))
            .collect(),
    }
}

pub fn impl_specialize_all(
    specialize_path: &Path,
    ast: &DeriveInput,
    field_info: &[FieldInfo],
) -> TokenStream {
    todo!()
}

pub fn impl_specialize_specific(
    specialize_path: &Path,
    ast: &DeriveInput,
    field_info: &[FieldInfo],
    target_path: &Path,
) -> TokenStream {
    let key_elems: Vec<Type> = field_info
        .iter()
        .filter_map(|field_info| field_info.key_ty(specialize_path, target_path))
        .collect();
    let specialize_stmts: Vec<Stmt> = field_info
        .iter()
        .map(|field_info| field_info.specialize_stmt(specialize_path, target_path))
        .collect();

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::Specialize<#target_path> for #struct_name #type_generics #where_clause {
            type Key = (#(#key_elems),*);

            fn specialize(&self, key: Self::Key, descriptor: &mut <#target_path as #specialize_path::SpecializeTarget>::Descriptor) {
                #(#specialize_stmts)*
            }
        }
    })
}
