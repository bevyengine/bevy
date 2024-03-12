use proc_macro::Span;
use syn::{punctuated::Punctuated, token::Comma, Data, DataStruct, Error, Field, Fields};

/// Get the fields of a data structure if that structure is a struct with named fields;
/// otherwise, return a compile error that points to the site of the macro invocation.
pub fn get_struct_fields(data: &Data) -> syn::Result<&Punctuated<Field, Comma>> {
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => Ok(&fields.named),
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => Ok(&fields.unnamed),
        _ => Err(Error::new(
            // This deliberately points to the call site rather than the structure
            // body; marking the entire body as the source of the error makes it
            // impossible to figure out which `derive` has a problem.
            Span::call_site().into(),
            "Only structs are supported",
        )),
    }
}
