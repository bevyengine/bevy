fn main() {
    println!(
        "cargo:rustc-env=PROC_ARTIFACT_DIR={}",
        std::env::var("OUT_DIR").unwrap()
    )
}
