use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Data, DataEnum, DataUnion, Error,
    Field, Fields,
};

/// Get the fields of a data structure if that structure is a struct with named fields;
/// otherwise, return a compile error that points to the site of the macro invocation.
///
/// `meta` should be the name of the macro calling this function.
pub fn get_struct_fields<'a>(
    data: &'a Data,
    meta: &str,
) -> syn::Result<&'a Punctuated<Field, Comma>> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => Ok(&fields_named.named),
            Fields::Unnamed(fields_unnamed) => Ok(&fields_unnamed.unnamed),
            Fields::Unit => Ok(const { &Punctuated::new() }),
        },
        Data::Enum(DataEnum { enum_token, .. }) => Err(Error::new(
            enum_token.span(),
            format!("#[{meta}] only supports structs, not enums"),
        )),
        Data::Union(DataUnion { union_token, .. }) => Err(Error::new(
            union_token.span(),
            format!("#[{meta}] only supports structs, not unions"),
        )),
    }
}
