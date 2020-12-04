use crate::modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Path};

pub fn derive_render_resource(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast.attrs);

    let bevy_render_path: Path = get_path(&modules.bevy_render);
    let bevy_asset_path: Path = get_path(&modules.bevy_asset);
    let bevy_core_path: Path = get_path(&modules.bevy_core);
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #bevy_render_path::renderer::RenderResource for #struct_name {
            fn resource_type(&self) -> Option<#bevy_render_path::renderer::RenderResourceType> {
                Some(#bevy_render_path::renderer::RenderResourceType::Buffer)
            }
            fn write_buffer_bytes(&self, buffer: &mut [u8]) {
                use #bevy_core_path::Bytes;
                self.write_bytes(buffer);
            }
            fn buffer_byte_len(&self) -> Option<usize> {
                use #bevy_core_path::Bytes;
                Some(self.byte_len())
            }
            fn texture(&self) -> Option<&#bevy_asset_path::Handle<#bevy_render_path::texture::Texture>> {
                None
            }

        }
    })
}
