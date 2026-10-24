mod attributes;
mod auto_register;
mod data;
mod field_attributes;
mod primitive;
mod type_data;
mod type_path;
mod type_registration;
mod validation;

use bevy_macro_utils::{fq_std::FQOption, BevyManifest};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Attribute, DeriveInput, Expr, ExprLit, Generics, Ident, Lit, LitStr,
    MacroDelimiter, Meta, Path,
};

use crate::reflect::{
    attributes::ContainerAttributes,
    data::{ReflectData, ReflectFields, ReflectVariant},
    type_path::{type_path, type_path_strings},
    type_registration::type_registration,
    validation::{definition_check, reject_packed},
};

#[cfg(feature = "auto_register_static")]
pub(crate) use auto_register::load_type_registrations;
pub(crate) use primitive::impl_reflect_primitive;

const REFLECT_ATTRIBUTE_NAME: &str = "reflect";
const TYPE_PATH_ATTRIBUTE_NAME: &str = "type_path";

/// Implements reflection for a type defined in the calling crate.
pub(crate) fn derive_reflect(ast: DeriveInput) -> Result<TokenStream, syn::Error> {
    reflect_impl(ast, ReflectTarget::Local)
}

/// Implements reflection for a type defined in another crate.
pub(crate) fn impl_reflect(mut ast: DeriveInput) -> Result<TokenStream, syn::Error> {
    let module_path = take_type_path_attribute(&mut ast.attrs)?.ok_or_else(|| {
        syn::Error::new(
            ast.ident.span(),
            format_args!(
                "`impl_reflect!` requires a `#[{TYPE_PATH_ATTRIBUTE_NAME} = \"path::to::module\"]` attribute"
            ),
        )
    })?;
    reflect_impl(ast, ReflectTarget::Foreign { module_path })
}

/// What the reflection impls are generated for.
enum ReflectTarget {
    /// A type defined in the calling crate (`#[derive(Reflect)]`).
    Local,
    /// A re-statement of a type defined in the module at `module_path` of another crate, which the
    /// calling crate can implement `Typed` for (`impl_reflect!`).
    Foreign { module_path: LitStr },
}

/// Generates the reflection impls for `ast` (see [`ReflectTarget`]).
fn reflect_impl(
    mut ast: DeriveInput,
    reflect_target: ReflectTarget,
) -> Result<TokenStream, syn::Error> {
    let bevy_reflect = BevyManifest::shared(|manifest| manifest.get_path("bevy_reflect2"));
    let container_attributes = parse_container_attributes(&ast.attrs)?;
    // Opaque types don't reflect their fields, so they can be packed
    if !container_attributes.opaque {
        reject_packed(&ast.attrs)?;
    }
    let ignored_params = ignored_params(&ast.generics, &container_attributes)?;
    // Custom where clause predicates (`#[reflect(where ...)]`) apply to every generated impl
    if !container_attributes.where_predicates.is_empty() {
        ast.generics
            .make_where_clause()
            .predicates
            .extend(container_attributes.where_predicates.iter().cloned());
    }
    let data = ReflectData::parse(&ast)?;
    let module_path = match &reflect_target {
        ReflectTarget::Local => quote! {::core::module_path!()},
        ReflectTarget::Foreign { module_path } => quote! {#module_path},
    };

    let extended_where_clause = extend_where_clause(
        &ast.generics,
        quote! {#bevy_reflect::Typed},
        &ignored_params,
    );
    let opaque = container_attributes.opaque;
    let (kind, kind_declarations) = match &data {
        // Opaque types don't reflect their fields
        _ if opaque => (
            quote! {#bevy_reflect::Kind::Primitive(#bevy_reflect::Primitive::Opaque)},
            quote! {},
        ),
        ReflectData::Struct(fields) => (struct_kind(fields, &bevy_reflect), quote! {}),
        ReflectData::Enum(variants) => {
            enum_kind(&ast, variants, &bevy_reflect, &extended_where_clause)
        }
    };
    // Derived types are checked by the compiler, but re-stated foreign types could have the wrong
    // fields or field types, which would make the reflected type information unsound. Opaque types
    // don't describe their fields, so there is nothing to check.
    let definition_check = match reflect_target {
        _ if opaque => quote! {},
        ReflectTarget::Local => quote! {},
        ReflectTarget::Foreign { .. } => definition_check(&data, &bevy_reflect),
    };

    let type_ident = &ast.ident;
    let type_ident_str = type_ident.to_string();
    let (impl_generics, type_generics, _) = &ast.generics.split_for_impl();
    let target = quote! {#type_ident #type_generics};
    let (generic_type_path_impl, full_type_path, short_type_path) = type_path_strings(
        &target,
        &ast.generics,
        &bevy_reflect,
        &extended_where_clause,
        &type_ident_str,
        &module_path,
        &ignored_params,
    );
    let type_path = type_path(
        &bevy_reflect,
        &module_path,
        &full_type_path,
        &short_type_path,
        type_ident_str.len(),
    );

    let vtable = container_attributes.vtable(&bevy_reflect);
    // Registering a type registers the types of its reflected fields, so the registration requires
    // the field types that depend on type parameters to be registrable (the type parameters
    // themselves only need to be `Typed`). Opaque types don't register their fields.
    let dependencies = if opaque {
        Vec::new()
    } else {
        data.reflected_field_types()
    };
    let registration_where_clause = {
        let mut generics = ast.generics.clone();
        let params = generics
            .type_params()
            .map(|param| param.ident.clone())
            .collect::<Vec<_>>();
        let predicates = dependencies
            .iter()
            .filter(|ty| mentions_any(ty.to_token_stream(), &params))
            .map(|ty| -> syn::WherePredicate {
                syn::parse_quote!(#ty: #bevy_reflect::GetTypeRegistration)
            })
            .collect::<Vec<_>>();
        if !predicates.is_empty() {
            generics.make_where_clause().predicates.extend(predicates);
        }
        extend_where_clause(&generics, quote! {#bevy_reflect::Typed}, &ignored_params)
    };
    let type_registration = type_registration(
        &target,
        &ast.generics,
        &bevy_reflect,
        &registration_where_clause,
        &container_attributes.type_data,
        &dependencies,
    );
    let auto_registration = match reflect_target {
        ReflectTarget::Local => {
            auto_register::auto_registration(&ast, &bevy_reflect, &container_attributes)
        }
        ReflectTarget::Foreign { .. } => quote! {},
    };
    // SAFETY: `TYPE`, kind, layout, and vtable are generated from the definition of `Self`
    Ok(quote! {
        unsafe impl #impl_generics #bevy_reflect::Typed for #target #extended_where_clause {
            const TYPE: &'static #bevy_reflect::Type = &{
                #definition_check
                let kind = #kind;
                let vtable = #vtable;
                unsafe {
                    #bevy_reflect::Type::new(
                        ::core::any::TypeId::of::<Self>(),
                        kind,
                        Some(::core::alloc::Layout::new::<Self>()),
                        vtable,
                        #type_path,
                    )
                }
            };
        }

        #generic_type_path_impl
        #type_registration
        #auto_registration
        #kind_declarations
    })
}

fn struct_kind(struct_fields: &ReflectFields, bevy_reflect: &Path) -> TokenStream {
    let field_kind = struct_fields.field_kind(bevy_reflect);
    let mut fields = Vec::new();
    let mut ignored_fields = Vec::new();
    for field in &struct_fields.fields {
        let name = field.name();
        let member = &field.member;
        if field.attributes.ignore {
            let vtable = field
                .attributes
                .ignored_field_vtable(field.field, bevy_reflect);
            ignored_fields.push(quote! {
                #bevy_reflect::IgnoredField {
                    name: #name,
                    offset: ::core::mem::offset_of!(Self, #member),
                    vtable: #vtable,
                }
            });
        } else {
            let field_ty = &field.field.ty;
            fields.push(quote! {
                #bevy_reflect::Field {
                    name: #name,
                    offset: ::core::mem::offset_of!(Self, #member),
                    ty: #bevy_reflect::TypeRef(#bevy_reflect::Type::of::<#field_ty>),
                }
            });
        }
    }
    // The field arrays are declared outside of the `unsafe` block, so that the custom functions in
    // ignored field vtables are not unintentionally allowed to perform unsafe operations.
    //
    // SAFETY: `fields` and `ignored_fields` contain every field of `Self`, with offsets, types, and
    // vtables generated from its definition, so every other byte is padding.
    quote! {{
        let fields = &[#(#fields),*];
        let ignored_fields = &[#(#ignored_fields),*];
        #bevy_reflect::Kind::Struct(unsafe {
            #bevy_reflect::Struct::new(
                fields,
                #field_kind,
                #bevy_reflect::StructKind::Struct,
                ignored_fields,
                #bevy_reflect::UncoveredBytes::Padding,
            )
        })
    }}
}

fn enum_kind(
    ast: &DeriveInput,
    reflect_variants: &[ReflectVariant],
    bevy_reflect: &Path,
    extended_where_clause: &TokenStream,
) -> (TokenStream, TokenStream) {
    let mut variants = Vec::with_capacity(reflect_variants.len());
    let mut variant_field_offsets = Vec::with_capacity(reflect_variants.len());
    let mut variant_indices = Vec::with_capacity(reflect_variants.len());
    let mut variant_constructors = Vec::with_capacity(reflect_variants.len());
    for (index, variant) in reflect_variants.iter().enumerate() {
        let variant_ident = &variant.variant.ident;

        let mut fields = Vec::new();
        let mut ignored_fields = Vec::new();
        // Every field is bound by reference, to compute its offset from `self`
        let mut field_patterns = Vec::new();
        // The offset of each reflected field, followed by the offset of each ignored field, which
        // matches the indices of `EnumAccess::field_offset`
        let mut field_offsets = Vec::new();
        let mut ignored_field_offsets = Vec::new();
        let mut field_initializers = Vec::new();
        let mut constructible = true;
        for field in &variant.fields.fields {
            let member = &field.member;
            let name = field.name();
            let binding = &field.binding;
            field_patterns.push(field.pattern());
            let offset = quote! {
                ::core::ptr::from_ref(#binding).addr() - ::core::ptr::from_ref(self).addr()
            };
            if field.attributes.ignore {
                let vtable = field
                    .attributes
                    .ignored_field_vtable(field.field, bevy_reflect);
                ignored_fields.push(quote! {
                    #bevy_reflect::EnumIgnoredField {
                        name: #name,
                        vtable: #vtable,
                    }
                });
                ignored_field_offsets.push(offset);
                match field.attributes.default_value(field.field, bevy_reflect) {
                    Some(value) => field_initializers.push(quote! { #member: #value }),
                    None => constructible = false,
                }
            } else {
                let field_ty = &field.field.ty;
                fields.push(quote! {
                    #bevy_reflect::EnumField {
                        name: #name,
                        ty: #bevy_reflect::TypeRef(#bevy_reflect::Type::of::<#field_ty>),
                    }
                });
                let field_index = field_offsets.len();
                field_offsets.push(offset);
                // Fields are initialized in declaration order, so `__fields` is called in the
                // order of `Variant::fields`
                field_initializers.push(quote! {
                    #member: <dyn #bevy_reflect::Reflect>::take::<#field_ty>(__fields(#field_index)?).ok()?
                });
            }
        }
        field_offsets.append(&mut ignored_field_offsets);
        let field_offset_cases = field_offsets
            .iter()
            .enumerate()
            .map(|(field_index, offset)| quote! { #field_index => #FQOption::Some(#offset) });
        variant_field_offsets.push(quote! {
            Self::#variant_ident { #(#field_patterns),* } => match index {
                #(#field_offset_cases,)*
                _ => #FQOption::None,
            }
        });
        variant_indices.push(quote! {
            Self::#variant_ident { .. } => #index
        });
        variant_constructors.push(if constructible {
            quote! { #index => #FQOption::Some(Self::#variant_ident { #(#field_initializers),* }) }
        } else {
            quote! { #index => #FQOption::None }
        });

        let field_kind = variant.fields.field_kind(bevy_reflect);
        let name = variant_ident.to_string();
        variants.push(quote! {
            #bevy_reflect::Variant {
                name: #name,
                field_kind: #field_kind,
                fields: &[#(#fields),*],
                ignored_fields: &[#(#ignored_fields),*],
            }
        });
    }
    // The variants and constructor are declared outside of the `unsafe` block, so that custom
    // default functions of ignored fields are not unintentionally allowed to perform unsafe
    // operations.
    //
    // SAFETY: `variants` is generated from the definition of `Self` in declaration order, with every
    // field, `Self` implements `EnumAccess` for it, and `construct` only returns values of `Self`
    // with the requested variant
    let kind = quote! {{
        let variants = &[#(#variants),*];
        let construct: #bevy_reflect::ConstructEnumFn = |__variant, __fields| {
            let value: #FQOption<Self> = match __variant {
                #(#variant_constructors,)*
                _ => #FQOption::None,
            };
            value.map(|value| {
                #bevy_reflect::__macro_exports::Box::new(value)
                    as #bevy_reflect::__macro_exports::Box<dyn #bevy_reflect::Reflect>
            })
        };
        #bevy_reflect::Kind::Enum(unsafe {
            #bevy_reflect::Enum::new(variants, #bevy_reflect::cast_enum_access::<Self>, construct)
        })
    }};
    let self_access = if reflect_variants.is_empty() {
        quote! {*self}
    } else {
        quote! {self}
    };
    let type_ident = &ast.ident;
    let (impl_generics, type_generics, _) = &ast.generics.split_for_impl();
    let kind_declarations = quote! {
        // SAFETY: `variant` and `field_offset` match the order of the generated `Enum::variants`. Field offsets are computed from fields bound by reference into
        // `self`, with reflected fields followed by ignored fields.
        unsafe impl #impl_generics #bevy_reflect::EnumAccess for #type_ident #type_generics #extended_where_clause {
            fn variant(&self) -> usize {
                match #self_access {
                    #(#variant_indices,)*
                }
            }

            fn field_offset(&self, index: usize) -> #FQOption<usize> {
                match #self_access {
                    #(#variant_field_offsets,)*
                }
            }
        }
    };

    (kind, kind_declarations)
}

/// Extends the where clause of `generics` with `constraint` for each type parameter. The `ignored`
/// parameters (`#[reflect(ignore_params(...))]`, see [`ignored_params`]) are only required to be
/// `'static`.
fn extend_where_clause(
    generics: &Generics,
    constraint: TokenStream,
    ignored: &[Ident],
) -> TokenStream {
    let where_clause = generics.where_clause.as_ref();
    if generics.type_params().next().is_some() {
        let where_clause_typed = generics.type_params().map(|p| {
            let ident = &p.ident;
            if ignored.contains(ident) {
                quote!(#ident: 'static)
            } else {
                quote!(#ident: #constraint)
            }
        });
        if let Some(where_clause) = where_clause {
            let predicates = where_clause
                .predicates
                .iter()
                .map(ToTokens::to_token_stream)
                .chain(where_clause_typed);
            quote!(where #(#predicates),*)
        } else {
            quote!(where #(#where_clause_typed),*)
        }
    } else {
        quote!(#where_clause)
    }
}

/// Returns the type parameters ignored with `#[reflect(ignore_params(...))]`. Ignored parameters
/// are not required to be `Typed` (ex: the type of a closure stored in an ignored field), and their
/// type paths come from [`core::any::type_name`].
fn ignored_params(
    generics: &Generics,
    container_attributes: &ContainerAttributes,
) -> Result<Vec<Ident>, syn::Error> {
    for param in &container_attributes.ignore_params {
        if !generics.type_params().any(|p| p.ident == *param) {
            return Err(syn::Error::new(
                param.span(),
                format_args!("`{param}` is not a type parameter of this type"),
            ));
        }
    }
    Ok(container_attributes.ignore_params.clone())
}

/// Removes the `#[type_path = "..."]` attribute from `attrs` and returns its value (if it exists).
fn take_type_path_attribute(attrs: &mut Vec<Attribute>) -> Result<Option<LitStr>, syn::Error> {
    let Some(index) = attrs
        .iter()
        .position(|attr| attr.path().is_ident(TYPE_PATH_ATTRIBUTE_NAME))
    else {
        return Ok(None);
    };
    let attr = attrs.remove(index);
    match &attr.meta {
        Meta::NameValue(name_value) => match &name_value.value {
            Expr::Lit(ExprLit {
                lit: Lit::Str(lit), ..
            }) => Ok(Some(lit.clone())),
            value => Err(syn::Error::new(value.span(), "expected a string literal")),
        },
        meta => Err(syn::Error::new(
            meta.span(),
            format_args!("expected `#[{TYPE_PATH_ATTRIBUTE_NAME} = \"...\"]`"),
        )),
    }
}

/// Returns whether `tokens` contain any of the `idents` (ex: whether a type mentions a type
/// parameter).
fn mentions_any(tokens: TokenStream, idents: &[Ident]) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => idents.contains(&ident),
        proc_macro2::TokenTree::Group(group) => mentions_any(group.stream(), idents),
        _ => false,
    })
}

fn parse_container_attributes(attrs: &[Attribute]) -> Result<ContainerAttributes, syn::Error> {
    let mut container_attributes = ContainerAttributes::default();
    for attr in attrs {
        match &attr.meta {
            Meta::List(meta_list) if meta_list.path.is_ident(REFLECT_ATTRIBUTE_NAME) => {
                if let MacroDelimiter::Paren(_) = meta_list.delimiter {
                    container_attributes.parse_meta_list(meta_list)?;
                } else {
                    return Err(syn::Error::new(
                        meta_list.delimiter.span().join(),
                        format_args!(
                            "`#[{REFLECT_ATTRIBUTE_NAME}(\"...\")]` must use parentheses `(` and `)`"
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(container_attributes)
}
