extern crate cc;

fn main() {
    cc::Build::new()
        .file("libmikktspace/mikktspace.c")
        .file("libmikktspace/mikktspace.c")
        .compile("mikktspace");
    println!("cargo:rustc-link-lib=static=mikktspace");
}
