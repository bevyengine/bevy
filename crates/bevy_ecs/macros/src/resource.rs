use bevy_ecs_macro_logic::component::{DeriveComponent, StorageAttribute, StorageTy};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path};

pub fn derive_resource(ast: &mut DeriveInput) -> TokenStream {
    let bevy_ecs: Path = crate::bevy_ecs_path();
    // A resource is just a `Component` that can also be stored as a singleton and registered in resource storage.
    // The `IsResource` marker that makes it visible to the resource APIs is added by the
    // `insert_resource`/`init_resource` pathways, *not* as a required component here, so
    // that the same type can still be used as an ordinary component (see #24591, #24592).
    let derive_component = match DeriveComponent::parse(ast, StorageAttribute::Disallowed) {
        Ok(value) => value,
        Err(e) => return e.into_compile_error(),
    };

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
