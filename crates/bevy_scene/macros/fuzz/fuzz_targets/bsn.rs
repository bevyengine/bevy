#![no_main]

use bevy_scene_macros_fuzz::{_bsn::types::BsnRoot, too_deep, try_codegen};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if too_deep(data) {
        return;
    }
    let Ok(tokens) = data.parse::<proc_macro2::TokenStream>() else {
        return;
    };
    let _ = try_codegen::<BsnRoot>(tokens);
});
