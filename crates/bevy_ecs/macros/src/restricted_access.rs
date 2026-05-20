use bevy_ecs_macro_logic::component::{DeriveComponent, StorageAttribute, StorageTy};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

pub fn derive_restricted_access(input: TokenStream) -> TokenStream {
    let component_input = input.clone();
    let mut component_ast = parse_macro_input!(component_input as DeriveInput);
    let derive_component = match DeriveComponent::parse(&component_ast, StorageAttribute::Allowed) {
        Ok(value) => value.with_restricted_access(),
        Err(e) => return e.into_compile_error().into(),
    };
    let bevy_ecs_path: Path = crate::bevy_ecs_path();
    let component_impl =
        match derive_component.impl_component(&mut component_ast, &bevy_ecs_path, StorageTy::Table)
        {
            Ok(value) => value,
            Err(err) => return err.into_compile_error().into(),
        };
    let mut ast = parse_macro_input!(input as DeriveInput);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: #bevy_ecs_path::component::Component<Mutability = #bevy_ecs_path::component::RestrictedMutable> });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        #component_impl

        impl #impl_generics #bevy_ecs_path::component::RestrictedAccess for #struct_name #type_generics #where_clause {
        }
    })
}
