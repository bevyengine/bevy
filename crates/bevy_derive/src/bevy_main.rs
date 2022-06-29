use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn bevy_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    assert!(
        input.sig.ident == "main",
        "`bevy_main` can only be used on a function called 'main'.",
    );

    TokenStream::from(quote! {
        // use ndk-glue macro to create an activity: https://github.com/rust-windowing/android-ndk-rs/tree/master/ndk-macro
        #[cfg(target_os = "android")]
        #[cfg_attr(target_os = "android", bevy::ndk_glue::main(backtrace = "on", ndk_glue = "bevy::ndk_glue"))]
        fn android_main() {
            main()
        }

        #[no_mangle]
        #[cfg(target_os = "ios")]
        extern "C" fn main_rs() {
            main();
        }

        #[allow(unused)]
        #input
    })
}
