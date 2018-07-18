extern crate cc;

fn main() {
    cc::Build::new()
        .file("libmikktspace/mikktspace.h")
        .file("libmikktspace/mikktspace.c")
        .compile("mikktspace");
}
