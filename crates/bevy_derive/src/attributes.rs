use darling::FromMeta;
use syn::{Attribute, Data, DataStruct, Field, Fields};

pub fn get_field_attributes<'a, T, TArgs>(
    attribute_name: &str,
    data: &'a Data,
) -> Vec<(&'a Field, T)>
where
    T: Default,
    TArgs: FromMeta + Into<T> + Default,
{
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    fields
        .iter()
        .map(|f| {
            (
                f,
                f.attrs
                    .iter()
                    .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == attribute_name)
                    .map(|a| {
                        TArgs::from_meta(&a.parse_meta().unwrap())
                            .unwrap_or_else(|_err| TArgs::default())
                    })
                    .unwrap_or_else(|| TArgs::default())
                    .into(),
            )
        })
        .collect::<Vec<(&Field, T)>>()
}

pub fn get_attributes<'a, T, TArgs>(attribute_name: &str, attrs: &[Attribute]) -> T
where
    T: Default,
    TArgs: FromMeta + Into<T> + Default,
{
    attrs
        .iter()
        .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == attribute_name)
        .map(|a| TArgs::from_meta(&a.parse_meta().unwrap()).unwrap_or_else(|_err| TArgs::default()))
        .unwrap_or_else(|| TArgs::default())
        .into()
}
