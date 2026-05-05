use crate::{derive_data::ReflectDerive, remote::generate_remote_assertions};
use quote::quote;

/// Generates an anonymous block containing compile-time assertions.
pub(crate) fn impl_assertions(derive_data: &ReflectDerive) -> proc_macro2::TokenStream {
    let mut output = quote!();

    if let Some(assertions) = generate_remote_assertions(derive_data) {
        output.extend(assertions);
    }

    output
}
