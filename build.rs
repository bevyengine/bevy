
extern crate cmake;

fn main() {
    let dst = cmake::build("libmikktspace");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=mikktspace");
}

