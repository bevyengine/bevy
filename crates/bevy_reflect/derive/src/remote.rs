use crate::{
    derive_data::{ReflectImplSource, ReflectProvenance, ReflectTraitToImpl},
    from_reflect,
    ident::ident_or_index,
    impls,
    impls::impl_assertions,
    ReflectDerive, REFLECT_ATTRIBUTE_NAME,
};
use bevy_macro_utils::fq_std::FQOption;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    token::PathSep,
    DeriveInput, ExprPath, Generics, Member, PathArguments, Type, TypePath,
};

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

    let assertions = impl_assertions(&derive_data);
    let definition_assertions = generate_remote_definition_assertions(&derive_data);

    let reflect_remote_impl = impl_reflect_remote(&derive_data, &remote_ty);

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
        ReflectDerive::Opaque(meta) => (
            impls::impl_opaque(&meta),
            if meta.from_reflect().should_auto_derive() {
                Some(from_reflect::impl_opaque(&meta))
            } else {
                None
            },
        ),
    };

    TokenStream::from(quote! {
        #wrapper_definition

        const _: () = {
            #reflect_remote_impl

            #reflect_impls

            #from_reflect_impl

            #definition_assertions

            #assertions
        };
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
        #[doc(hidden)]
        #vis struct #ident #ty_generics (pub #remote_ty) #where_clause;
    }
}

/// Generates the implementation of the `ReflectRemote` trait for the given derive data and remote type.
///
/// # Note to Developers
///
/// The `ReflectRemote` trait could likely be made with default method implementations.
/// However, this makes it really easy for a user to accidentally implement this trait in an unsafe way.
/// To prevent this, we instead generate the implementation through a macro using this function.
fn impl_reflect_remote(input: &ReflectDerive, remote_ty: &TypePath) -> proc_macro2::TokenStream {
    let bevy_reflect_path = input.meta().bevy_reflect_path();

    let type_path = input.meta().type_path();
    let (impl_generics, ty_generics, where_clause) =
        input.meta().type_path().generics().split_for_impl();

    let where_reflect_clause = input
        .where_clause_options()
        .extend_where_clause(where_clause);

    quote! {
        // SAFE: The generated wrapper type is guaranteed to be valid and repr(transparent) over the remote type.
        impl #impl_generics #bevy_reflect_path::ReflectRemote for #type_path #ty_generics #where_reflect_clause {
            type Remote = #remote_ty;

            fn as_remote(&self) -> &Self::Remote {
                &self.0
            }
            fn as_remote_mut(&mut self) -> &mut Self::Remote {
                &mut self.0
            }
            fn into_remote(self) -> Self::Remote
            {
                // SAFE: The wrapper type should be repr(transparent) over the remote type
                unsafe {
                    // Unfortunately, we have to use `transmute_copy` to avoid a compiler error:
                    // ```
                    // error[E0512]: cannot transmute between types of different sizes, or dependently-sized types
                    // |
                    // |                 core::mem::transmute::<A, B>(a)
                    // |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                    // |
                    // = note: source type: `A` (this type does not have a fixed size)
                    // = note: target type: `B` (this type does not have a fixed size)
                    // ```
                    ::core::mem::transmute_copy::<Self, Self::Remote>(
                        // `ManuallyDrop` is used to prevent double-dropping `self`
                        &::core::mem::ManuallyDrop::new(self)
                    )
                }
            }

            fn as_wrapper(remote: &Self::Remote) -> &Self {
                // SAFE: The wrapper type should be repr(transparent) over the remote type
                unsafe { ::core::mem::transmute::<&Self::Remote, &Self>(remote) }
            }
            fn as_wrapper_mut(remote: &mut Self::Remote) -> &mut Self {
                // SAFE: The wrapper type should be repr(transparent) over the remote type
                unsafe { ::core::mem::transmute::<&mut Self::Remote, &mut Self>(remote) }
            }
            fn into_wrapper(remote: Self::Remote) -> Self
            {
                // SAFE: The wrapper type should be repr(transparent) over the remote type
                unsafe {
                    // Unfortunately, we have to use `transmute_copy` to avoid a compiler error:
                    // ```
                    // error[E0512]: cannot transmute between types of different sizes, or dependently-sized types
                    // |
                    // |                 core::mem::transmute::<A, B>(a)
                    // |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                    // |
                    // = note: source type: `A` (this type does not have a fixed size)
                    // = note: target type: `B` (this type does not have a fixed size)
                    // ```
                    ::core::mem::transmute_copy::<Self::Remote, Self>(
                        // `ManuallyDrop` is used to prevent double-dropping `self`
                        &::core::mem::ManuallyDrop::new(remote)
                    )
                }
            }
        }
    }
}

/// Generates compile-time assertions for remote fields.
///
/// This prevents types from using an invalid remote type.
/// this works by generating a struct, `RemoteFieldAssertions`, with methods that
/// will result in compile-time failure if types are mismatched.
/// The output of this function is best placed within an anonymous context to maintain hygiene.
///
/// # Example
///
/// The following would fail to compile due to an incorrect `#[reflect(remote = ...)]` value.
///
/// ```ignore
/// mod external_crate {
///     pub struct TheirFoo(pub u32);
///     pub struct TheirBar(pub i32);
/// }
///
/// #[reflect_remote(external_crate::TheirFoo)]
/// struct MyFoo(pub u32);
/// #[reflect_remote(external_crate::TheirBar)]
/// struct MyBar(pub i32);
///
/// #[derive(Reflect)]
/// struct MyStruct {
///   #[reflect(remote = MyBar)] // ERROR: expected type `TheirFoo` but found struct `TheirBar`
///   foo: external_crate::TheirFoo
/// }
/// ```
pub(crate) fn generate_remote_assertions(
    derive_data: &ReflectDerive,
) -> Option<proc_macro2::TokenStream> {
    struct RemoteAssertionData<'a> {
        ident: Member,
        variant: Option<&'a Ident>,
        ty: &'a Type,
        generics: &'a Generics,
        remote_ty: &'a Type,
    }

    let bevy_reflect_path = derive_data.meta().bevy_reflect_path();

    let fields: Box<dyn Iterator<Item = RemoteAssertionData>> = match derive_data {
        ReflectDerive::Struct(data)
        | ReflectDerive::TupleStruct(data)
        | ReflectDerive::UnitStruct(data) => Box::new(data.active_fields().filter_map(|field| {
            field
                .attrs
                .remote
                .as_ref()
                .map(|remote_ty| RemoteAssertionData {
                    ident: ident_or_index(field.data.ident.as_ref(), field.declaration_index),
                    variant: None,
                    ty: &field.data.ty,
                    generics: data.meta().type_path().generics(),
                    remote_ty,
                })
        })),
        ReflectDerive::Enum(data) => Box::new(data.variants().iter().flat_map(|variant| {
            variant.active_fields().filter_map(|field| {
                field
                    .attrs
                    .remote
                    .as_ref()
                    .map(|remote_ty| RemoteAssertionData {
                        ident: ident_or_index(field.data.ident.as_ref(), field.declaration_index),
                        variant: Some(&variant.data.ident),
                        ty: &field.data.ty,
                        generics: data.meta().type_path().generics(),
                        remote_ty,
                    })
            })
        })),

        _ => return None,
    };

    let assertions = fields
        .map(move |field| {
            let ident = if let Some(variant) = field.variant {
                format_ident!("{}__{}", variant, field.ident)
            } else {
                match field.ident {
                    Member::Named(ident) => ident,
                    Member::Unnamed(index) => format_ident!("field_{}", index),
                }
            };
            let (impl_generics, _, where_clause) = field.generics.split_for_impl();

            let where_reflect_clause = derive_data
                .where_clause_options()
                .extend_where_clause(where_clause);

            let ty = &field.ty;
            let remote_ty = field.remote_ty;
            let assertion_ident = format_ident!("assert__{}__is_valid_remote", ident);

            let span = create_assertion_span(remote_ty.span());

            quote_spanned! {span=>
                #[allow(non_snake_case)]
                #[allow(clippy::multiple_bound_locations)]
                fn #assertion_ident #impl_generics () #where_reflect_clause {
                    let _: <#remote_ty as #bevy_reflect_path::ReflectRemote>::Remote = (|| -> #FQOption<#ty> {
                        None
                    })().unwrap();
                }
            }
        })
        .collect::<proc_macro2::TokenStream>();

    if assertions.is_empty() {
        None
    } else {
        Some(quote! {
            struct RemoteFieldAssertions;

            impl RemoteFieldAssertions {
                #assertions
            }
        })
    }
}

/// Generates compile-time assertions that ensure a remote wrapper definition matches up with the
/// remote type it's wrapping.
///
/// Note: This currently results in "backwards" error messages like:
///
/// ```ignore
/// expected: <WRAPPER_FIELD_TYPE>
/// found: <REMOTE_FIELD_TYPE>
/// ```
///
/// Ideally it would be the other way around, but there's no easy way of doing this without
/// generating a copy of the struct/enum definition and using that as the base instead of the remote type.
fn generate_remote_definition_assertions(derive_data: &ReflectDerive) -> proc_macro2::TokenStream {
    let meta = derive_data.meta();
    let self_ident = format_ident!("__remote__");
    let self_ty = derive_data.remote_ty().unwrap().type_path();
    let self_expr_path = derive_data.remote_ty().unwrap().as_expr_path().unwrap();
    let (impl_generics, _, where_clause) = meta.type_path().generics().split_for_impl();

    let where_reflect_clause = derive_data
        .where_clause_options()
        .extend_where_clause(where_clause);

    let assertions = match derive_data {
        ReflectDerive::Struct(data)
        | ReflectDerive::TupleStruct(data)
        | ReflectDerive::UnitStruct(data) => {
            let mut output = proc_macro2::TokenStream::new();

            for field in data.fields() {
                let field_member =
                    ident_or_index(field.data.ident.as_ref(), field.declaration_index);
                let field_ty = &field.data.ty;
                let span = create_assertion_span(field_ty.span());

                output.extend(quote_spanned! {span=>
                    #self_ident.#field_member = (|| -> #FQOption<#field_ty> {None})().unwrap();
                });
            }

            output
        }
        ReflectDerive::Enum(data) => {
            let variants = data.variants().iter().map(|variant| {
                let ident = &variant.data.ident;

                let mut output = proc_macro2::TokenStream::new();

                if variant.fields().is_empty() {
                    return quote!(#self_expr_path::#ident => {});
                }

                for field in variant.fields() {
                    let field_member =
                        ident_or_index(field.data.ident.as_ref(), field.declaration_index);
                    let field_ident = format_ident!("field_{}", field_member);
                    let field_ty = &field.data.ty;
                    let span = create_assertion_span(field_ty.span());

                    output.extend(quote_spanned! {span=>
                        #self_expr_path::#ident {#field_member: mut #field_ident, ..} => {
                            #field_ident =  (|| -> #FQOption<#field_ty> {None})().unwrap();
                        }
                    });
                }

                output
            });

            quote! {
                match #self_ident {
                    #(#variants)*
                }
            }
        }
        ReflectDerive::Opaque(_) => {
            // No assertions needed since there are no fields to check
            proc_macro2::TokenStream::new()
        }
    };

    quote! {
        const _: () = {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            #[allow(unused_assignments)]
            #[allow(unreachable_patterns)]
            #[allow(clippy::multiple_bound_locations)]
            fn assert_wrapper_definition_matches_remote_type #impl_generics (mut #self_ident: #self_ty) #where_reflect_clause {
                #assertions
            }
        };
    }
}

/// Creates a span located around the given one, but resolves to the assertion's context.
///
/// This should allow the compiler to point back to the line and column in the user's code,
/// while still attributing the error to the macro.
fn create_assertion_span(span: Span) -> Span {
    Span::call_site().located_at(span)
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
