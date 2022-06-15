use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Index, Member, Type};

pub fn derive_deref(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let (field_member, field_type) = match get_inner_field(&ast, false) {
        Ok(items) => items,
        Err(err) => {
            return err.into_compile_error().into();
        }
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics ::std::ops::Deref for #ident #ty_generics #where_clause {
            type Target = #field_type;

            fn deref(&self) -> &Self::Target {
                &self.#field_member
            }
        }
    })
}

pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let (field_member, _) = match get_inner_field(&ast, true) {
        Ok(items) => items,
        Err(err) => {
            return err.into_compile_error().into();
        }
    };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics ::std::ops::DerefMut for #ident #ty_generics #where_clause {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.#field_member
            }
        }
    })
}

fn get_inner_field(ast: &DeriveInput, is_mut: bool) -> syn::Result<(Member, &Type)> {
    match &ast.data {
        Data::Struct(data_struct) if data_struct.fields.len() == 1 => {
            let field = data_struct.fields.iter().next().unwrap();
            let member = field
                .ident
                .as_ref()
                .map(|name| Member::Named(name.clone()))
                .unwrap_or_else(|| Member::Unnamed(Index::from(0)));
            Ok((member, &field.ty))
        }
        _ => {
            let msg = if is_mut {
                "DerefMut can only be derived for structs with a single field"
            } else {
                "Deref can only be derived for structs with a single field"
            };
            Err(syn::Error::new(Span::call_site().into(), msg))
        }
    }
}
