use std::env;

fn main() {
    // Note: bevy_audio fix
    // This would not be needed if using cpal directly, but bevy_audio uses
    // rodio which uses cpal, and its not clear why, but c++ lib is not properly
    // linked, this is a workaround for now.  This can also be fixed by enabling
    // the shared-stdcxx feature for oboe in cpal
    // https://github.com/RustAudio/cpal/issues/720
    // https://github.com/RustAudio/cpal/issues/563
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    if let Ok("android") = target_os.as_ref().map(|x| &**x) {
        println!("cargo:rustc-link-lib=c++_shared");
    }
}
