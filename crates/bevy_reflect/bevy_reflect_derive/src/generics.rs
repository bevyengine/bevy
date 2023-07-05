use crate::field_attributes::IGNORE_ALL_ATTR;
use crate::REFLECT_ATTRIBUTE_NAME;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, GenericParam, Generics, ImplGenerics, Meta, TypeGenerics, TypeParam, WhereClause,
};

pub(crate) struct ReflectGenerics {
    generics: Generics,
    attrs: Vec<GenericParamAttr>,
}

impl ReflectGenerics {
    pub const EMPTY: &ReflectGenerics = &ReflectGenerics {
        generics: Generics {
            gt_token: None,
            lt_token: None,
            where_clause: None,
            params: Punctuated::new(),
        },
        attrs: Vec::new(),
    };

    pub fn new(generics: &Generics) -> Result<Self, syn::Error> {
        let mut generics = generics.clone();

        let attrs = generics
            .params
            .iter_mut()
            .map(GenericParamAttr::from_param)
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(Self { generics, attrs })
    }

    pub fn params(&self) -> impl Iterator<Item = (&GenericParam, &GenericParamAttr)> {
        self.generics.params.iter().zip(self.attrs.iter())
    }

    pub fn type_params(&self) -> impl Iterator<Item = (&TypeParam, &GenericParamAttr)> {
        self.params().filter_map(|(param, attr)| match param {
            GenericParam::Type(param) => Some((param, attr)),
            _ => None,
        })
    }

    pub fn split_for_impl(&self) -> (ImplGenerics, TypeGenerics, Option<&WhereClause>) {
        self.generics.split_for_impl()
    }
}

#[derive(Default, Clone)]
pub(crate) struct GenericParamAttr {
    pub ignore: bool,
}

impl GenericParamAttr {
    fn from_param(param: &mut GenericParam) -> Result<Self, syn::Error> {
        match param {
            GenericParam::Type(param) => Self::from_type_attrs(&mut param.attrs),
            GenericParam::Lifetime(param) => Self::from_lifetime_attrs(&mut param.attrs),
            GenericParam::Const(param) => Self::from_const_attrs(&mut param.attrs),
        }
    }

    fn from_type_attrs(attrs: &mut Vec<Attribute>) -> Result<Self, syn::Error> {
        let mut args = Self::default();
        let mut errors: Option<syn::Error> = None;

        attrs.retain(|attr| {
            if attr.path().is_ident(REFLECT_ATTRIBUTE_NAME) {
                if let Err(err) = parse_meta(&mut args, &attr.meta) {
                    if let Some(ref mut error) = errors {
                        error.combine(err);
                    } else {
                        errors = Some(err);
                    }
                }

                false
            } else {
                true
            }
        });

        if let Some(error) = errors {
            Err(error)
        } else {
            Ok(args)
        }
    }

    fn from_lifetime_attrs(attrs: &mut [Attribute]) -> Result<Self, syn::Error> {
        match attrs
            .iter()
            .find(|attr| attr.path().is_ident(REFLECT_ATTRIBUTE_NAME))
        {
            Some(attr) => Err(syn::Error::new_spanned(
                attr,
                format!(
                    "{REFLECT_ATTRIBUTE_NAME} attributes cannot be used on lifetime parameters"
                ),
            )),
            None => Ok(Self::default()),
        }
    }

    fn from_const_attrs(attrs: &mut [Attribute]) -> Result<Self, syn::Error> {
        match attrs
            .iter()
            .find(|attr| attr.path().is_ident(REFLECT_ATTRIBUTE_NAME))
        {
            Some(attr) => Err(syn::Error::new_spanned(
                attr,
                format!("{REFLECT_ATTRIBUTE_NAME} attributes cannot be used on const parameters"),
            )),
            None => Ok(Self::default()),
        }
    }
}

fn parse_meta(args: &mut GenericParamAttr, meta: &Meta) -> Result<(), syn::Error> {
    let meta = meta.require_list()?;
    meta.parse_nested_meta(|meta| {
        if meta.path.is_ident(IGNORE_ALL_ATTR) {
            args.ignore = true;
            Ok(())
        } else {
            Err(syn::Error::new_spanned(
                &meta.path,
                format!("unknown attribute '{}'", meta.path.to_token_stream()),
            ))
        }
    })
}
