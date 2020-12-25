extern crate proc_macro;

mod animated_asset;
mod animated_component;
mod animated_properties;
mod help;
mod modules;

use proc_macro::TokenStream;

#[proc_macro_derive(AnimatedComponent, attributes(animated))]
pub fn derive_animated_component(input: TokenStream) -> TokenStream {
    animated_component::derive_animated_component(input)
}

/// Used to implement the `AnimatedComponent` for an struct defined externally;
/// Only useful inside the bevy_animation crate.
#[doc(hidden)]
#[proc_macro]
pub fn animated_component(input: TokenStream) -> TokenStream {
    animated_component::derive_animated_component(input)
}

#[proc_macro_derive(AnimatedAsset, attributes(animated))]
pub fn derive_animated_asset(input: TokenStream) -> TokenStream {
    animated_asset::derive_animated_asset(input)
}

/// Used to implement the `AnimatedAsset` for an struct defined externally;
/// Only useful inside the bevy_animation crate.
#[doc(hidden)]
#[proc_macro]
pub fn animated_asset(input: TokenStream) -> TokenStream {
    animated_asset::derive_animated_asset(input)
}

///////////////////////////////////////////////////////////////////////////////

#[proc_macro_derive(AnimatedProperties, attributes(animated))]
pub fn derive_animated_properties(input: TokenStream) -> TokenStream {
    animated_properties::derive_animated_properties(input)
}

/// Used to implement the `AnimatedAsset` for an struct defined externally;
/// Only useful inside the bevy_animation crate.
#[doc(hidden)]
#[proc_macro]
pub fn animated_properties(input: TokenStream) -> TokenStream {
    animated_properties::derive_animated_properties(input)
}

///////////////////////////////////////////////////////////////////////////////

// TODO: LerpValue and Blend
