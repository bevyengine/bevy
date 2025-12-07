extern crate embed_resource;
fn main() {
    println!("cargo:rerun-if-changed=icons.rc");
    println!("cargo:rerun-if-changed=favicon.ico");
    embed_resource::compile("icons.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
