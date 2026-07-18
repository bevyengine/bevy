use bevy_ecs_macro_logic::component::{DeriveComponent, StorageAttribute, StorageTy};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path};

pub fn derive_resource(ast: &mut DeriveInput) -> TokenStream {
    let bevy_ecs: Path = crate::bevy_ecs_path();
    let mut derive_component = match DeriveComponent::parse(ast, StorageAttribute::Disallowed) {
        Ok(value) => value,
        Err(e) => return e.into_compile_error(),
    };

    // We add an extra insert hook to the resource
    derive_component.additional_insert_hook = Some(quote!(#bevy_ecs::resource::on_resource_insert));

    // We add the component_id existence check here to avoid recursive init during required components initialization.
    derive_component.additional_requires.push(quote! {
        required_components.register_required::<#bevy_ecs::resource::IsResource>(|| #bevy_ecs::resource::IsResource);
    });

    let component_impl = match derive_component.impl_component(ast, &bevy_ecs, StorageTy::SparseSet)
    {
        Ok(value) => value,
        Err(err) => return err.into_compile_error(),
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    quote! {
        #component_impl
        impl #impl_generics #bevy_ecs::resource::Resource for #struct_name #type_generics #where_clause {
        }
    }
}
