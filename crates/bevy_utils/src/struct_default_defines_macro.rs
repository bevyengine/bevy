/// Defines simple default struct initialization macros with the given names and paths.
#[macro_export]
macro_rules! define_struct_default_macros {
    ($($name:ident: $path:path),*) => {
        $(
            bevy_utils::define_struct_default_macro!($name, $path);
        )*
    };
}
