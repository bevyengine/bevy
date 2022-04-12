use bevy_macro_utils::get_named_struct_fields;
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{parse_quote, DeriveInput, Path};

pub fn emit(input: DeriveInput) -> TokenStream {
    let bevy_crevice_path = crate::bevy_crevice_path();

    let fields = match get_named_struct_fields(&input.data) {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error(),
    };

    let base_trait_path: Path = parse_quote!(#bevy_crevice_path::glsl::Glsl);
    let struct_trait_path: Path = parse_quote!(#bevy_crevice_path::glsl::GlslStruct);

    let name = input.ident;
    let name_str = Literal::string(&name.to_string());

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let glsl_fields = fields.named.iter().map(|field| {
        let field_ty = &field.ty;
        let field_name_str = Literal::string(&field.ident.as_ref().unwrap().to_string());
        let field_as = quote! {<#field_ty as #bevy_crevice_path::glsl::GlslArray>};

        quote! {
            s.push_str("\t");
            s.push_str(#field_as::NAME);
            s.push_str(" ");
            s.push_str(#field_name_str);
            <#field_as::ArraySize as #bevy_crevice_path::glsl::DimensionList>::push_to_string(s);
            s.push_str(";\n");
        }
    });

    quote! {
        unsafe impl #impl_generics #base_trait_path for #name #ty_generics #where_clause {
            const NAME: &'static str = #name_str;
        }

        unsafe impl #impl_generics #struct_trait_path for #name #ty_generics #where_clause {
            fn enumerate_fields(s: &mut String) {
                #( #glsl_fields )*
            }
        }
    }
}
