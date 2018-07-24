extern crate cc;

fn main() {
    cc::Build::new()
        .file("libmikktspace/mikktspace.c")
        .include("libmikktspace")
        .compile("mikktspace");
}
