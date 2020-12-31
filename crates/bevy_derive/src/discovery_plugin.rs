use std::{
    fs::{File, OpenOptions},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use proc_macro2::TokenStream;
use quote::quote;
use ron::Value;
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use syn::{parse_macro_input, Attribute, DeriveInput, Item, LitStr};

use crate::modules::{self, get_path};

#[derive(Default)]
struct DiscoveryAttributes {
    pub root: String,
}

fn take_attr_value(attrs: &[Attribute], key: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| *a.path.get_ident().as_ref().unwrap() == key)?
        .parse_args::<LitStr>()
        .as_ref()
        .map(LitStr::value)
        .ok()
}

pub fn derive_discovery_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = modules::get_modules(&ast.attrs);
    let root_filename =
        take_attr_value(&ast.attrs, "root").unwrap_or_else(|| "src/main.rs".to_owned());
    let path = PathBuf::from(root_filename);
    let mut manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    manifest_dir.push(path);
    let path = manifest_dir;

    let mut hasher = FxHasher::default();
    path.to_str().unwrap().hash(&mut hasher);
    let hash = hasher.finish();

    let out_dir = env!("PROC_ARTIFACT_DIR");
    let mut cache_dir = PathBuf::from(out_dir);
    cache_dir.push(PathBuf::from(format!("discovery_cache_{:x}", hash)));
    let cache_path = cache_dir.with_extension("ron");

    let mut cache = File::open(&cache_path)
        .ok()
        .and_then(|mut file| {
            let mut cache_str = String::new();
            file.read_to_string(&mut cache_str)
                .expect("Unable to read cache");
            cache_str.parse::<Value>().ok()
        })
        .unwrap_or_else(|| Value::Map(ron::Map::new()))
        .into_rust::<FxHashMap<PathBuf, CacheEntry>>()
        .unwrap_or_default();

    let mut ts = TokenStream::new();
    search_file_cache(&path, &mut cache, &mut ts, &quote! { self });

    let mut cache_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(&cache_path)
        .unwrap();

    cache_file
        .write_all(
            ron::ser::to_string_pretty(&cache, Default::default())
                .unwrap()
                .as_bytes(),
        )
        .expect("Cannot write to cache");

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

fn search_file_cache(
    filepath: &Path,
    cache: &mut FxHashMap<PathBuf, CacheEntry>,
    ts: &mut TokenStream,
    module_path: &TokenStream,
) {
    let fp = filepath.display().to_string();
    let last_modified = filepath
        .metadata()
        .unwrap_or_else(|e| panic!("cannot read metadata for {}: {}", fp, e))
        .modified()
        .expect("cannot read last modified")
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    if let Some((filepath, entry)) = cache.remove_entry(filepath) {
        let module_path = syn::parse_str::<syn::Path>(&entry.module_path).unwrap();
        let module_path = &quote! { #module_path };
        if last_modified == entry.last_modified {
            for entry in entry.fn_paths.iter() {
                let path = syn::parse_str::<syn::Path>(&entry.path).expect("Broken cache");
                if let Some(stage) = &entry.stage {
                    let stage = syn::parse_str::<TokenStream>(stage).unwrap();
                    ts.extend(quote! { .add_system_to_stage(#stage, #path.system()) });
                } else {
                    ts.extend(quote! { .add_system(#path.system()) });
                }
            }

            for file in entry.referenced_files.iter() {
                search_file_cache(&file, cache, ts, module_path);
            }
            cache.insert(filepath, entry);
        } else {
            search_file(
                filepath,
                module_path,
                ts,
                &entry.search_directory,
                cache,
                last_modified,
            );
        }
    } else {
        let search_path = match filepath
            .with_extension("")
            .file_name()
            .and_then(|s| s.to_str())
        {
            Some("mod") | Some("lib") | Some("main") => filepath.parent().unwrap().to_owned(),
            _ => filepath.with_extension(""),
        };

        search_file(
            filepath.to_owned(),
            module_path,
            ts,
            &search_path,
            cache,
            last_modified,
        )
    }
}

fn search_file(
    filepath: PathBuf,
    module_path: &TokenStream,
    ts: &mut TokenStream,
    search_path: &Path,
    cache: &mut FxHashMap<PathBuf, CacheEntry>,
    last_modified: Duration,
) {
    let mut file = File::open(&filepath).expect("File not found");

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    let syntax = syn::parse_file(&src).expect("Unable to parse file");
    let csr = search_contents(
        &syntax.items,
        &quote! { #module_path },
        ts,
        search_path,
        cache,
    );

    cache.insert(
        filepath,
        CacheEntry {
            fn_paths: csr.direct_additions,
            referenced_files: csr.direct_referenced_paths,
            search_directory: search_path.to_owned(),
            last_modified,
            module_path: module_path.to_string(),
        },
    );
}

#[derive(Default)]
struct ContentSearchResult {
    direct_additions: Vec<SystemEntry>,
    direct_referenced_paths: Vec<PathBuf>,
}

fn search_contents(
    content: &[Item],
    module_path: &TokenStream,
    ts: &mut TokenStream,
    search_path: &Path,
    cache: &mut FxHashMap<PathBuf, CacheEntry>,
) -> ContentSearchResult {
    let mut csr = ContentSearchResult::default();
    for item in content.iter() {
        match item {
            Item::Fn(f) => {
                if let Some(a) = f.attrs.iter().find(|a| a.path == get_path("system")) {
                    let ident = &f.sig.ident;
                    let stage = a.parse_args::<TokenStream>().ok();
                    let path = &quote! { #module_path::#ident };
                    let addition = if let Some(stage) = &stage {
                        quote! { .add_system_to_stage( #stage, #path.system()) }
                    } else {
                        quote! { .add_system(#path.system()) }
                    };
                    csr.direct_additions.push(SystemEntry {
                        path: path.to_string(),
                        stage: stage.as_ref().map(TokenStream::to_string),
                    });
                    ts.extend(addition);
                }
            }
            Item::Mod(modd) => {
                let mut path = module_path.to_owned();
                let ident = &modd.ident;
                path.extend(quote! { ::#ident });
                let mut dir = search_path.to_owned();
                dir.extend(&[&ident.to_string()]);

                match &modd.content {
                    Some((_, content)) => {
                        let mut subcsr = search_contents(content, &path, ts, &dir, cache);
                        csr.direct_additions
                            .extend(subcsr.direct_additions.drain(..));
                        csr.direct_referenced_paths
                            .extend(subcsr.direct_referenced_paths.drain(..));
                    }
                    None => {
                        let mut filepath = dir;
                        if !filepath.with_extension("rs").exists() {
                            filepath.extend(&["mod"]);
                        }
                        filepath.set_extension("rs");
                        search_file_cache(&filepath, cache, ts, &path);
                        csr.direct_referenced_paths.push(filepath);
                    }
                }
            }
            _ => continue,
        }
    }
    csr
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    last_modified: Duration,
    referenced_files: Vec<PathBuf>,
    fn_paths: Vec<SystemEntry>,
    module_path: String,
    search_directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct SystemEntry {
    path: String,
    stage: Option<String>,
}
