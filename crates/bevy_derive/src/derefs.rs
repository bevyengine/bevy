use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Index, Member, Type};

const DEREF: &str = "Deref";
const DEREF_MUT: &str = "DerefMut";
const DEREF_ATTR: &str = "deref";

pub fn derive_deref(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let (field_member, field_type) = match get_deref_field(&ast, false) {
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
    let (field_member, _) = match get_deref_field(&ast, true) {
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

fn get_deref_field(ast: &DeriveInput, is_mut: bool) -> syn::Result<(Member, &Type)> {
    let deref_kind = if is_mut { DEREF_MUT } else { DEREF };
    let deref_attr_str = format!("`#[{DEREF_ATTR}]`");

    match &ast.data {
        Data::Struct(data_struct) if data_struct.fields.is_empty() => Err(syn::Error::new(
            Span::call_site().into(),
            format!("{deref_kind} cannot be derived on field-less structs"),
        )),
        Data::Struct(data_struct) if data_struct.fields.len() == 1 => {
            let field = data_struct.fields.iter().next().unwrap();
            let member = to_member(field, 0);
            Ok((member, &field.ty))
        }
        Data::Struct(data_struct) => {
            let mut selected_field: Option<(Member, &Type)> = None;
            for (index, field) in data_struct.fields.iter().enumerate() {
                for attr in &field.attrs {
                    if !attr.meta.path().is_ident(DEREF_ATTR) {
                        continue;
                    }

                    attr.meta.require_path_only()?;

                    if selected_field.is_some() {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!(
                                "{deref_attr_str} attribute can only be used on a single field"
                            ),
                        ));
                    }

                    let member = to_member(field, index);
                    selected_field = Some((member, &field.ty));
                }
            }

            if let Some(selected_field) = selected_field {
                Ok(selected_field)
            } else {
                Err(syn::Error::new(
                    Span::call_site().into(),
                    format!("deriving {deref_kind} on multi-field structs requires one field to have the {deref_attr_str} attribute"),
                ))
            }
        }
        _ => Err(syn::Error::new(
            Span::call_site().into(),
            format!("{deref_kind} can only be derived on structs"),
        )),
    }
}

fn to_member(field: &Field, index: usize) -> Member {
    field
        .ident
        .as_ref()
        .map(|name| Member::Named(name.clone()))
        .unwrap_or_else(|| Member::Unnamed(Index::from(index)))
}
