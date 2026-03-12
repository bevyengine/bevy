#![cfg_attr(docsrs, feature(doc_cfg))]

//! Macros for deriving `States` and `SubStates` traits.

extern crate proc_macro;

mod states;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

/// Derive macro for the `States` trait.
///
/// Defines world-wide states for a finite-state machine. The type must also
/// derive `Clone`, `PartialEq`, `Eq`, `Hash`, `Debug`, and `Default` (the
/// default variant is the starting state).
///
/// See the `States` trait docs for full explanation.
///
/// ```ignore
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
///     Paused,
/// }
/// ```
#[proc_macro_derive(States, attributes(states))]
pub fn derive_states(input: TokenStream) -> TokenStream {
    states::derive_states(input)
}

/// Derive macro for the `SubStates` trait.
///
/// Defines a sub-state that only exists when a source state matches a specific
/// value. While active, sub-states can be manually modified unlike `ComputedStates`.
///
/// See the `SubStates` trait docs for full explanation.
///
/// ```ignore
/// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// // This sub-state only exists when AppState is InGame.
/// #[source(AppState = AppState::InGame)]
/// enum GamePhase {
///     #[default]
///     Setup,
///     Battle,
///     Conclusion,
/// }
/// ```
#[proc_macro_derive(SubStates, attributes(states, source))]
pub fn derive_substates(input: TokenStream) -> TokenStream {
    states::derive_substates(input)
}

pub(crate) fn bevy_state_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_state"))
}
