use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Item, LitStr};

use crate::modules::{self, get_path};

#[derive(Default)]
struct DiscoveryAttributes {
    pub root: String,
}

pub fn derive_discovery_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = modules::get_modules(&ast.attrs);
    let root_filename = ast
        .attrs
        .iter()
        .find(|a| *a.path.get_ident().as_ref().unwrap() == "root")
        .expect("set search root")
        .parse_args::<LitStr>()
        .as_ref()
        .map(LitStr::value)
        .unwrap_or("src/main.rs".to_string());

    let mut ts = TokenStream::new();

    let path = PathBuf::from(root_filename);
    let mut manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    manifest_dir.push(path);
    
    let path = manifest_dir;

    let mut file = File::open(&path).expect("Unable to open file");
    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");
    let syntax = syn::parse_file(&src).expect("Unable to parse file");

    let path = match path.with_extension("").file_name().and_then(|s| s.to_str()) {
        Some("mod") | Some("lib") | Some("main") => path.parent().unwrap().to_owned(),
        _ => path,
    };

    search_contents(&syntax.items, quote! { self }, &mut ts, &path);

    let app_path = modules::get_path(&modules.bevy_app);
    let input_ident = &ast.ident;

    (quote! {
        impl #app_path::DiscoveryPlugin for #input_ident {
            fn build(&self, app: &mut AppBuilder) {
                app#ts;
            }
        }
    })
    .into()
}

fn search_contents(
    content: &[Item],
    path: TokenStream,
    ts: &mut TokenStream,
    search_directory: &Path,
) {
    for item in content.iter() {
        match item {
            Item::Fn(f) => {
                if f.attrs.iter().any(|a| a.path == get_path("system")) {
                    let ident = &f.sig.ident;
                    ts.extend(quote! {
                        .add_system(#path::#ident.system())
                    })
                }
            }
            Item::Mod(modd) => {
                let mut path = path.clone();
                let ident = &modd.ident;
                path.extend(quote! { ::#ident });
                match &modd.content {
                    Some((_, content)) => {
                        search_contents(content, path, ts, search_directory);
                    }
                    None => {
                        let mut dir = search_directory.to_owned();
                        dir.extend(&[&ident.to_string()]);
                        let mut file = File::open(dir.with_extension("rs"))
                            .ok()
                            .map_or_else(
                                || {
                                    let mut extend = dir.to_owned();
                                    extend.extend(&["mod"]);
                                    File::open(&extend.with_extension("rs")).ok()
                                },
                                Some,
                            )
                            .expect("Unable to open file");

                        let mut src = String::new();
                        file.read_to_string(&mut src).expect("Unable to read file");

                        let syntax = syn::parse_file(&src).expect("Unable to parse file");

                        search_contents(&syntax.items, path, ts, &dir);
                    }
                }
            }
            _ => continue,
        }
    }
}
