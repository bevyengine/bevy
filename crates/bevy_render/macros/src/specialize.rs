use bevy_macro_utils::fq_std::{FQDefault, FQResult};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Data, DataStruct, DeriveInput, Expr, Fields, Ident, Index, Member, Meta, MetaList, Path, Stmt,
    Token, Type, WherePredicate,
};

const SPECIALIZE_ATTR_IDENT: &str = "specialize";
const SPECIALIZE_ALL_IDENT: &str = "all";

const KEY_ATTR_IDENT: &str = "key";
const KEY_DEFAULT_IDENT: &str = "default";

const BASE_DESCRIPTOR_ATTR_IDENT: &str = "base_descriptor";

enum SpecializeImplTargets {
    All,
    Specific(Vec<Path>),
}

impl Parse for SpecializeImplTargets {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let paths = input.parse_terminated(Path::parse, Token![,])?;
        if paths
            .first()
            .is_some_and(|p| p.is_ident(SPECIALIZE_ALL_IDENT))
        {
            Ok(SpecializeImplTargets::All)
        } else {
            Ok(SpecializeImplTargets::Specific(paths.into_iter().collect()))
        }
    }
}

#[derive(Clone)]
enum Key {
    Whole,
    Default,
    Index(Index),
    Custom(Expr),
}

impl Key {
    fn expr(&self) -> Expr {
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

#[derive(Clone)]
struct FieldInfo {
    ty: Type,
    member: Member,
    key: Key,
    use_base_descriptor: bool,
}

impl FieldInfo {
    fn key_ty(&self, specialize_path: &Path, target_path: &Path) -> Option<Type> {
        let ty = &self.ty;
        matches!(self.key, Key::Whole | Key::Index(_))
            .then_some(parse_quote!(<#ty as #specialize_path::Specialize<#target_path>>::Key))
    }

    fn specialize_stmt(&self, specialize_path: &Path, target_path: &Path) -> Stmt {
        let FieldInfo {
            ty, member, key, ..
        } = &self;
        let key_expr = key.expr();
        parse_quote!(<#ty as #specialize_path::Specialize<#target_path>>::specialize(&self.#member, #key_expr, descriptor)?;)
    }

    fn specialize_predicate(&self, specialize_path: &Path, target_path: &Path) -> WherePredicate {
        let ty = &self.ty;
        if matches!(&self.key, Key::Default) {
            parse_quote!(#ty: #specialize_path::Specialize<#target_path, Key: #FQDefault>)
        } else {
            parse_quote!(#ty: #specialize_path::Specialize<#target_path>)
        }
    }

    fn has_base_descriptor_predicate(
        &self,
        specialize_path: &Path,
        target_path: &Path,
    ) -> WherePredicate {
        let ty = &self.ty;
        parse_quote!(#ty: #specialize_path::HasBaseDescriptor<#target_path>)
    }
}

fn get_field_info(fields: &Fields, targets: &SpecializeImplTargets) -> syn::Result<Vec<FieldInfo>> {
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
        let mut use_base_descriptor = false;
        for attr in &field.attrs {
            match &attr.meta {
                Meta::Path(path) if path.is_ident(&BASE_DESCRIPTOR_ATTR_IDENT) => {
                    use_base_descriptor = true;
                }
                Meta::List(MetaList { path, tokens, .. }) if path.is_ident(&KEY_ATTR_IDENT) => {
                    let owned_tokens = tokens.clone().into();
                    let Ok(parsed_key) = parse::<Key>(owned_tokens) else {
                        return Err(syn::Error::new(
                            attr.span(),
                            "Invalid key override attribute",
                        ));
                    };
                    key = parsed_key;
                    if matches!(
                        (&key, &targets),
                        (Key::Custom(_), SpecializeImplTargets::All)
                    ) {
                        return Err(syn::Error::new(
                            attr.span(),
                            "#[key(default)] is the only key override type allowed with #[specialize(all)]",
                        ));
                    }
                    use_key_field = false;
                }
                _ => {}
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
            use_base_descriptor,
        });
    }

    if used_count == 1 {
        field_info[single_index].key = Key::Whole;
    }

    Ok(field_info)
}

fn get_struct_fields<'a>(ast: &'a DeriveInput, derive_name: &str) -> syn::Result<&'a Fields> {
    match &ast.data {
        Data::Struct(DataStruct { fields, .. }) => Ok(fields),
        Data::Enum(data_enum) => Err(syn::Error::new(
            data_enum.enum_token.span(),
            format!("#[derive({derive_name})] only supports structs."),
        )),
        Data::Union(data_union) => Err(syn::Error::new(
            data_union.union_token.span(),
            format!("#[derive({derive_name})] only supports structs."),
        )),
    }
}

fn get_specialize_targets(
    ast: &DeriveInput,
    derive_name: &str,
) -> syn::Result<SpecializeImplTargets> {
    let specialize_attr = ast.attrs.iter().find_map(|attr| {
        if attr.path().is_ident(SPECIALIZE_ATTR_IDENT) {
            if let Meta::List(meta_list) = &attr.meta {
                return Some(meta_list);
            }
        }
        None
    });
    let Some(specialize_meta_list) = specialize_attr else {
        return Err(syn::Error::new(
            Span::call_site(),
            format!("#[derive({derive_name})] must be accompanied by #[specialize(..targets)].\n Example usages: #[specialize(RenderPipeline)], #[specialize(all)]")
        ));
    };
    parse::<SpecializeImplTargets>(specialize_meta_list.tokens.clone().into())
}

macro_rules! guard {
    ($expr: expr) => {
        match $expr {
            Ok(__val) => __val,
            Err(err) => return err.to_compile_error().into(),
        }
    };
}

pub fn impl_specialize(input: TokenStream) -> TokenStream {
    let bevy_render_path: Path = crate::bevy_render_path();
    let specialize_path = {
        let mut path = bevy_render_path.clone();
        path.segments.push(format_ident!("render_resource").into());
        path
    };

    let ecs_path = crate::bevy_ecs_path();

    let ast = parse_macro_input!(input as DeriveInput);
    let targets = guard!(get_specialize_targets(&ast, "Specialize"));
    let fields = guard!(get_struct_fields(&ast, "Specialize"));
    let field_info = guard!(get_field_info(fields, &targets));

    let base_descriptor_fields = field_info
        .iter()
        .filter(|field| field.use_base_descriptor)
        .collect::<Vec<_>>();

    if base_descriptor_fields.len() > 1 {
        return syn::Error::new(
            Span::call_site(),
            "Too many #[base_descriptor] attributes found. It must be present on exactly one field",
        )
        .into_compile_error()
        .into();
    }

    let base_descriptor_field = base_descriptor_fields.first().copied();

    match targets {
        SpecializeImplTargets::All => {
            let specialize_impl =
                impl_specialize_all(&specialize_path, &ecs_path, &ast, &field_info);
            let get_base_descriptor_impl = base_descriptor_field
                .map(|field_info| impl_get_base_descriptor_all(&specialize_path, &ast, field_info))
                .unwrap_or_default();
            [specialize_impl, get_base_descriptor_impl]
                .into_iter()
                .collect()
        }
        SpecializeImplTargets::Specific(targets) => {
            let specialize_impls = targets.iter().map(|target| {
                impl_specialize_specific(&specialize_path, &ecs_path, &ast, &field_info, target)
            });
            let get_base_descriptor_impls = targets.iter().filter_map(|target| {
                base_descriptor_field.map(|field_info| {
                    impl_get_base_descriptor_specific(&specialize_path, &ast, field_info, target)
                })
            });
            specialize_impls.chain(get_base_descriptor_impls).collect()
        }
    }
}

fn impl_specialize_all(
    specialize_path: &Path,
    ecs_path: &Path,
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
                .push(field.specialize_predicate(specialize_path, &target_path));
        }
    }

    let (_, type_generics, _) = ast.generics.split_for_impl();
    let (impl_generics, _, where_clause) = &generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::Specialize<#target_path> for #struct_name #type_generics #where_clause {
            type Key = (#(#key_elems),*);

            fn specialize(
                &self,
                key: Self::Key,
                descriptor: &mut <#target_path as #specialize_path::Specializable>::Descriptor
            ) -> #FQResult<#ecs_path::error::BevyError> {
                #(#specialize_stmts)*
            }
        }
    })
}

fn impl_specialize_specific(
    specialize_path: &Path,
    ecs_path: &Path,
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

            fn specialize(
                &self,
                key: Self::Key,
                descriptor: &mut <#target_path as #specialize_path::Specializable>::Descriptor
            ) -> #FQResult<#ecs_path::error::BevyError> {
                #(#specialize_stmts)*
            }
        }
    })
}

pub fn impl_get_base_descriptor(input: TokenStream) -> TokenStream {
    let bevy_render_path: Path = crate::bevy_render_path();
    let specialize_path = {
        let mut path = bevy_render_path.clone();
        path.segments.push(format_ident!("render_resource").into());
        path
    };

    let ast = parse_macro_input!(input as DeriveInput);
    let targets = guard!(get_specialize_targets(&ast, "Specialize"));
    let fields = guard!(get_struct_fields(&ast, "Specialize"));
    let field_info = guard!(get_field_info(fields, &targets));

    let base_descriptor_fields = field_info
        .iter()
        .filter(|field| field.use_base_descriptor)
        .collect::<Vec<_>>();

    let base_descriptor_field = if field_info.len() == 1 {
        field_info[0].clone()
    } else if base_descriptor_fields.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[base_descriptor] attribute not found. It must be present on exactly one field",
        )
        .into_compile_error()
        .into();
    } else if base_descriptor_fields.len() > 1 {
        return syn::Error::new(
            Span::call_site(),
            "Too many #[base_descriptor] attributes found. It must be present on exactly one field",
        )
        .into_compile_error()
        .into();
    } else {
        base_descriptor_fields[0].clone()
    };

    match targets {
        SpecializeImplTargets::All => {
            impl_get_base_descriptor_all(&specialize_path, &ast, &base_descriptor_field)
        }
        SpecializeImplTargets::Specific(targets) => targets
            .iter()
            .map(|target| {
                impl_get_base_descriptor_specific(
                    &specialize_path,
                    &ast,
                    &base_descriptor_field,
                    target,
                )
            })
            .collect(),
    }
}

fn impl_get_base_descriptor_specific(
    specialize_path: &Path,
    ast: &DeriveInput,
    base_descriptor_field_info: &FieldInfo,
    target_path: &Path,
) -> TokenStream {
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let field_ty = &base_descriptor_field_info.ty;
    let field_member = &base_descriptor_field_info.member;
    TokenStream::from(quote!(
        impl #impl_generics #specialize_path::GetBaseDescriptor<#target_path> for #struct_name #type_generics #where_clause {
            fn get_base_descriptor(&self) -> <#target_path as #specialize_path::Specializable>::Descriptor {
                <#field_ty as #specialize_path::HasBaseDescriptor<#target_path>>::base_descriptor(&self.#field_member)
            }
        }
    ))
}

fn impl_get_base_descriptor_all(
    specialize_path: &Path,
    ast: &DeriveInput,
    base_descriptor_field_info: &FieldInfo,
) -> TokenStream {
    let target_path = Path::from(format_ident!("T"));
    let struct_name = &ast.ident;
    let mut generics = ast.generics.clone();
    generics.params.insert(
        0,
        parse_quote!(#target_path: #specialize_path::Specializable),
    );

    let where_clause = generics.make_where_clause();
    where_clause.predicates.push(
        base_descriptor_field_info.has_base_descriptor_predicate(specialize_path, &target_path),
    );

    let (_, type_generics, _) = ast.generics.split_for_impl();
    let (impl_generics, _, where_clause) = &generics.split_for_impl();
    let field_ty = &base_descriptor_field_info.ty;
    let field_member = &base_descriptor_field_info.member;
    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::GetBaseDescriptor<#target_path> for #struct_name #type_generics #where_clause {
            fn get_base_descriptor(&self) -> <#target_path as #specialize_path::Specializable>::Descriptor {
                <#field_ty as #specialize_path::HasBaseDescriptor<#target_path>>::base_descriptor(&self.#field_member)
            }
        }
    })
}
