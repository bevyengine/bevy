mod glsl;
mod layout;

use proc_macro::TokenStream as CompilerTokenStream;

use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(AsStd140)]
pub fn derive_as_std140(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = layout::emit(input, "Std140", "std140", 16);

    CompilerTokenStream::from(expanded)
}

#[proc_macro_derive(AsStd430)]
pub fn derive_as_std430(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = layout::emit(input, "Std430", "std430", 0);

    CompilerTokenStream::from(expanded)
}

#[proc_macro_derive(GlslStruct)]
pub fn derive_glsl_struct(input: CompilerTokenStream) -> CompilerTokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = glsl::emit(input);

    CompilerTokenStream::from(expanded)
}
