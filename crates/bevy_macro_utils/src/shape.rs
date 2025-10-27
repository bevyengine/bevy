use syn::{spanned::Spanned, Data, DataEnum, DataUnion, Error, Fields};

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
