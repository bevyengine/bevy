fn main() {
    let search_path = std::env::current_dir().unwrap().join("fsr").join("lib");
    println!("cargo:rustc-link-search=native={}", search_path.display());

    #[cfg(debug_assertions)]
    {
        println!("cargo:rustc-link-lib=static=ffx_fsr2_api_x64d");
        println!("cargo:rustc-link-lib=static=ffx_fsr2_api_vk_x64d");
    }
    #[cfg(not(debug_assertions))]
    {
        println!("cargo:rustc-link-lib=static=ffx_fsr2_api_x64");
        println!("cargo:rustc-link-lib=static=ffx_fsr2_api_vk_x64");
    }
}
