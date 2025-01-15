use bevy_macro_utils::fq_std::FQDefault;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Data, DataStruct, DeriveInput, Expr, Ident, Index, Member, Meta, MetaList, Path, Stmt, Token,
    Type, WherePredicate,
};

const SPECIALIZE_ATTR_IDENT: &str = "specialize";
const SPECIALIZE_ALL_IDENT: &str = "all";

const KEY_ATTR_IDENT: &str = "key";
const KEY_DEFAULT_IDENT: &str = "default";

enum SpecializeImplTargets {
    All,
    Specific(Vec<Path>),
}

impl Parse for SpecializeImplTargets {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let paths = input.parse_terminated(Path::parse, Token![,])?;
        if paths[0].is_ident(SPECIALIZE_ALL_IDENT) {
            Ok(SpecializeImplTargets::All)
        } else {
            Ok(SpecializeImplTargets::Specific(paths.into_iter().collect()))
        }
    }
}

enum Key {
    Whole,
    Default,
    Index(Index),
    Custom(Expr),
}

impl Key {
    pub fn expr(&self) -> Expr {
        match self {
            Key::Whole => parse_quote!(key),
            Key::Default => parse_quote!(#FQDefault::default()),
            Key::Index(index) => {
                let member = Member::Unnamed(index.clone());
                parse_quote!(key.#member)
            }
            Key::Custom(expr) => expr.clone(),
        }
    }
}

const KEY_ERROR_MSG: &str = "Invalid key override. Must be either `default` or a valid Rust expression of the correct key type";

impl Parse for Key {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(ident) = input.parse::<Ident>() {
            if ident == KEY_DEFAULT_IDENT {
                Ok(Key::Default)
            } else {
                Err(syn::Error::new_spanned(ident, KEY_ERROR_MSG))
            }
        } else {
            input.parse::<Expr>().map(Key::Custom).map_err(|mut err| {
                err.extend(syn::Error::new(err.span(), KEY_ERROR_MSG));
                err
            })
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
        let ty = &self.ty;
        matches!(self.key, Key::Whole | Key::Index(_))
            .then_some(parse_quote!(<#ty as #specialize_path::Specialize<#target_path>>::Key))
    }

    pub fn specialize_stmt(&self, specialize_path: &Path, target_path: &Path) -> Stmt {
        let FieldInfo { ty, member, key } = &self;
        let key_expr = key.expr();
        parse_quote!(<#ty as #specialize_path::Specialize<#target_path>>::specialize(&self.#member, #key_expr, descriptor);)
    }

    pub fn predicate(&self, specialize_path: &Path, target_path: &Path) -> WherePredicate {
        let ty = &self.ty;
        if matches!(&self.key, Key::Default) {
            parse_quote!(#ty: #specialize_path::Specialize<#target_path, Key: #FQDefault>)
        } else {
            parse_quote!(#ty: #specialize_path::Specialize<#target_path>)
        }
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
            "#[derive(Specialize) must be accompanied by #[specialize(..targets)].\n Example usages: #[specialize(RenderPipeline)], #[specialize(all)]"
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
                    key = parse_macro_input!(owned_tokens as Key);
                    if matches!(
                        (&key, &targets),
                        (Key::Custom(_), SpecializeImplTargets::All)
                    ) {
                        return syn::Error::new(
                            tokens.span(),
                            "#[key(default)] is the only key override type allowed with #[specialize(all)]",
                        )
                        .into_compile_error()
                        .into();
                    }
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

fn impl_specialize_all(
    specialize_path: &Path,
    ast: &DeriveInput,
    field_info: &[FieldInfo],
) -> TokenStream {
    let target_path = Path::from(format_ident!("T"));
    let key_elems: Vec<Type> = field_info
        .iter()
        .filter_map(|field_info| field_info.key_ty(specialize_path, &target_path))
        .collect();
    let specialize_stmts: Vec<Stmt> = field_info
        .iter()
        .map(|field_info| field_info.specialize_stmt(specialize_path, &target_path))
        .collect();

    let struct_name = &ast.ident;
    let mut generics = ast.generics.clone();
    generics.params.insert(
        0,
        parse_quote!(#target_path: #specialize_path::Specializable),
    );

    if !field_info.is_empty() {
        let where_clause = generics.make_where_clause();
        for field in field_info {
            where_clause
                .predicates
                .push(field.predicate(specialize_path, &target_path));
        }
    }

    let (_, type_generics, _) = ast.generics.split_for_impl();
    let (impl_generics, _, where_clause) = &generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::Specialize<#target_path> for #struct_name #type_generics #where_clause {
            type Key = (#(#key_elems),*);

            fn specialize(&self, key: Self::Key, descriptor: &mut <#target_path as #specialize_path::Specializable>::Descriptor) {
                #(#specialize_stmts)*
            }
        }
    })
}

fn impl_specialize_specific(
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

            fn specialize(&self, key: Self::Key, descriptor: &mut <#target_path as #specialize_path::Specializable>::Descriptor) {
                #(#specialize_stmts)*
            }
        }
    })
}
