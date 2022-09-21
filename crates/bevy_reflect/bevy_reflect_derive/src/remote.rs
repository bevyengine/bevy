use crate::derive_data::{ReflectImplSource, ReflectProvenance, ReflectTraitToImpl};
use crate::{from_reflect, impls, ReflectDerive, REFLECT_ATTRIBUTE_NAME};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, TypePath};

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

    derive_data.set_remote(Some(&remote_ty));

    let (reflect_impls, from_reflect_impl) = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => (
            impls::impl_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::TupleStruct(struct_data) => (
            impls::impl_tuple_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_tuple_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::Enum(enum_data) => (
            impls::impl_enum(&enum_data),
            if enum_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_enum(&enum_data))
            } else {
                None
            },
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
