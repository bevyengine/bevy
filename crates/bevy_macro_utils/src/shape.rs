use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Data, DataEnum, DataUnion, Error,
    Field, Fields,
};

/// Get the fields of a data structure if that structure is a struct;
/// otherwise, return a compile error that points to the site of the macro invocation.
///
/// `meta` should be the name of the macro calling this function.
pub fn get_struct_fields<'a>(data: &'a Data, meta: &str) -> Result<&'a Fields, Error> {
    match data {
        Data::Struct(data_struct) => Ok(&data_struct.fields),
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

/// Return an error if `Fields` is not `Fields::Named`
pub fn require_named<'a>(fields: &'a Fields) -> Result<&'a Punctuated<Field, Comma>, Error> {
    if let Fields::Named(fields) = fields {
        Ok(&fields.named)
    } else {
        Err(Error::new(
            fields.span(),
            "Unnamed fields are not supported here",
        ))
    }
}
