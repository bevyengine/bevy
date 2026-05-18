#![no_main]

use bevy_scene_macros_fuzz::{bsn::types::BsnRoot, try_codegen};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let Ok(tokens) = data.parse::<proc_macro2::TokenStream>() else {
        return;
    };
    let _ = try_codegen::<BsnRoot>(tokens);
});
