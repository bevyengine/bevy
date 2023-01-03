use crate::derive_data::{
    EnumVariantFields, ReflectImplSource, ReflectProvenance, ReflectTraitToImpl, StructField,
};
use crate::utility::ident_or_index;
use crate::{from_reflect, impls, ReflectDerive, REFLECT_ATTRIBUTE_NAME};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::PathSep;
use syn::{parse_macro_input, DeriveInput, ExprPath, Generics, PathArguments, TypePath};

/// Generates the remote wrapper type and implements all the necessary traits.
pub(crate) fn reflect_remote(args: TokenStream, input: TokenStream) -> TokenStream {
    let remote_args = match syn::parse::<RemoteArgs>(args) {
        Ok(path) => path,
        Err(err) => return err.to_compile_error().into(),
    };

    let remote_ty = remote_args.remote_ty;

    let ast = parse_macro_input!(input as DeriveInput);
    let wrapper_definition = generate_remote_wrapper(&ast, &remote_ty);

    let mut derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source: ReflectImplSource::RemoteReflect,
            trait_: ReflectTraitToImpl::Reflect,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    derive_data.set_remote(Some(RemoteType::new(&remote_ty)));

    let (reflect_impls, from_reflect_impl, assertions) = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => (
            impls::impl_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_struct(&struct_data))
            } else {
                None
            },
            Some(generate_remote_field_assertions(
                struct_data.fields(),
                None,
                struct_data.meta().type_path().generics(),
            )),
        ),
        ReflectDerive::TupleStruct(struct_data) => (
            impls::impl_tuple_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_tuple_struct(&struct_data))
            } else {
                None
            },
            Some(generate_remote_field_assertions(
                struct_data.fields(),
                None,
                struct_data.meta().type_path().generics(),
            )),
        ),
        ReflectDerive::Enum(enum_data) => (
            impls::impl_enum(&enum_data),
            if enum_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_enum(&enum_data))
            } else {
                None
            },
            enum_data
                .variants()
                .iter()
                .map(|variant| match &variant.fields {
                    EnumVariantFields::Named(fields) | EnumVariantFields::Unnamed(fields) => {
                        Some(generate_remote_field_assertions(
                            fields,
                            Some(&variant.data.ident),
                            enum_data.meta().type_path().generics(),
                        ))
                    }
                    EnumVariantFields::Unit => None,
                })
                .collect(),
        ),
        _ => {
            return syn::Error::new(ast.span(), "cannot reflect a remote value type")
                .into_compile_error()
                .into()
        }
    };

    TokenStream::from(quote! {
        #wrapper_definition

        #reflect_impls

        #from_reflect_impl

        #assertions
    })
}

/// Generates the remote wrapper type.
///
/// # Example
///
/// If the supplied remote type is `Bar<T>`, then the wrapper type— named `Foo<T>`— would look like:
///
/// ```
/// # struct Bar<T>(T);
///
/// #[repr(transparent)]
/// struct Foo<T>(Bar<T>);
/// ```
fn generate_remote_wrapper(input: &DeriveInput, remote_ty: &TypePath) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let vis = &input.vis;
    let ty_generics = &input.generics;
    let where_clause = &input.generics.where_clause;
    let attrs = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident(REFLECT_ATTRIBUTE_NAME));

    quote! {
        #(#attrs)*
        #[repr(transparent)]
        #vis struct #ident #ty_generics (pub #remote_ty) #where_clause;
    }
}

/// Generates compile-time assertions for remote fields.
///
/// # Example
///
/// The following would fail to compile due to an incorrect `#[reflect(remote = "...")]` value.
///
/// ```ignore
/// mod external_crate {
///     pub struct TheirOuter {
///         pub inner: TheirInner,
///     }
///     pub struct TheirInner(pub String);
/// }
///
/// #[reflect_remote(external_crate::TheirOuter)]
/// struct MyOuter {
///     #[reflect(remote = "MyOuter")] // <- Note the mismatched type (it should be `MyInner`)
///     pub inner: external_crate::TheirInner,
/// }
///
/// #[reflect_remote(external_crate::TheirInner)]
/// struct MyInner(pub String);
/// ```
fn generate_remote_field_assertions(
    fields: &[StructField<'_>],
    variant: Option<&Ident>,
    generics: &Generics,
) -> proc_macro2::TokenStream {
    fields
        .iter()
        .filter(|field| field.attrs.remote.is_some())
        .map(|field| {
            let ident = if let Some(variant) = variant {
                format_ident!(
                    "{}__{}",
                    variant,
                    ident_or_index(field.data.ident.as_ref(), field.declaration_index)
                )
            } else {
                field
                    .data
                    .ident
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| format_ident!("field_{}", field.declaration_index))
            };

            let (impl_generics, _, where_clause) = generics.split_for_impl();
            let field_ty = &field.data.ty;
            let remote_ty = field.attrs.remote.as_ref().unwrap();
            let assertion_ident = format_ident!("assert__{}__is_valid_remote", ident);

            quote! {
                const _: () = {
                    struct RemoteFieldAssertions;

                    impl RemoteFieldAssertions {
                        #[allow(non_snake_case)]
                        fn #assertion_ident #impl_generics (#ident: #remote_ty) #where_clause {
                            let _: #field_ty = #ident.0;
                        }
                    }
                };
            }
        })
        .collect()
}

/// A reflected type's remote type.
///
/// This is a wrapper around [`TypePath`] that allows it to be paired with other remote-specific logic.
#[derive(Copy, Clone)]
pub(crate) struct RemoteType<'a> {
    path: &'a TypePath,
}

impl<'a> RemoteType<'a> {
    pub fn new(path: &'a TypePath) -> Self {
        Self { path }
    }

    /// Returns the [type path](TypePath) of this remote type.
    pub fn type_path(&self) -> &'a TypePath {
        self.path
    }

    /// Attempts to convert the [type path](TypePath) of this remote type into an [expression path](ExprPath).
    ///
    /// For example, this would convert `foo::Bar<T>` into `foo::Bar::<T>` to be used as part of an expression.
    ///
    /// This will return an error for types that are parenthesized, such as in `Fn() -> Foo`.
    pub fn as_expr_path(&self) -> Result<ExprPath, syn::Error> {
        let mut expr_path = self.path.clone();
        if let Some(segment) = expr_path.path.segments.last_mut() {
            match &mut segment.arguments {
                PathArguments::None => {}
                PathArguments::AngleBracketed(arg) => {
                    arg.colon2_token = Some(PathSep::default());
                }
                PathArguments::Parenthesized(arg) => {
                    return Err(syn::Error::new(
                        arg.span(),
                        "cannot use parenthesized type as remote type",
                    ))
                }
            }
        }

        Ok(ExprPath {
            path: expr_path.path,
            qself: expr_path.qself,
            attrs: Vec::new(),
        })
    }
}

/// Metadata from the arguments defined in the `reflect_remote` attribute.
///
/// The syntax for the arguments is: `#[reflect_remote(REMOTE_TYPE_PATH)]`.
struct RemoteArgs {
    remote_ty: TypePath,
}

impl Parse for RemoteArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            remote_ty: input.parse()?,
        })
    }
}
