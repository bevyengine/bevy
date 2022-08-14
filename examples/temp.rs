use std::path::Path;

fn main() {
    let p = Path::new(file!());
    println!(
        "{}/{}",
        p.parent().unwrap().to_string_lossy(),
        "render/pbr_types.wgsl"
    );
}
