use crate::{Reflect, ReflectRef, Struct, Tuple};

pub trait Enum: Reflect {
    fn variant(&self) -> EnumVariant<'_>;
    fn variant_mut(&mut self) -> EnumVariantMut<'_>;
    fn variant_info(&self) -> VariantInfo<'_>;
    fn iter_variants_info(&self) -> VariantInfoIter<'_>;
    fn get_index_name(&self, index: usize) -> Option<&str>;
    fn get_index_from_name(&self, name: &str) -> Option<usize>;
}

#[derive(PartialEq, Eq)]
pub struct VariantInfo<'a> {
    pub index: usize,
    pub name: &'a str,
}
pub struct VariantInfoIter<'a> {
    pub(crate) value: &'a dyn Enum,
    pub(crate) index: usize,
}

impl<'a> VariantInfoIter<'a> {
    pub fn new(value: &'a dyn Enum) -> Self {
        Self { value, index: 0 }
    }
}

impl<'a> Iterator for VariantInfoIter<'a> {
    type Item = VariantInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self
            .value
            .get_index_name(self.index)
            .map(|name| VariantInfo {
                index: self.index,
                name,
            });
        self.index += 1;
        value
    }
}

pub enum EnumVariant<'a> {
    Unit,
    NewType(&'a dyn Reflect),
    Tuple(&'a dyn Tuple),
    Struct(&'a dyn Struct),
}
pub enum EnumVariantMut<'a> {
    Unit,
    NewType(&'a mut dyn Reflect),
    Tuple(&'a mut dyn Tuple),
    Struct(&'a mut dyn Struct),
}

#[inline]
pub fn enum_partial_eq<E: Enum>(enum_a: &E, reflect_b: &dyn Reflect) -> Option<bool> {
    let enum_b = if let ReflectRef::Enum(e) = reflect_b.reflect_ref() {
        e
    } else {
        return Some(false);
    };

    if enum_a.variant_info() != enum_b.variant_info() {
        return Some(false);
    }

    let variant_b = enum_b.variant();
    match enum_a.variant() {
        EnumVariant::Unit => {
            if let EnumVariant::Unit = variant_b {
            } else {
                return Some(false);
            }
        }
        EnumVariant::NewType(t_a) => {
            if let EnumVariant::NewType(t_b) = variant_b {
                if let Some(false) | None = t_b.reflect_partial_eq(t_a) {
                    return Some(false);
                }
            } else {
                return Some(false);
            }
        }
        EnumVariant::Tuple(t_a) => {
            if let EnumVariant::Tuple(t_b) = variant_b {
                if let Some(false) | None = t_b.reflect_partial_eq(t_a.as_reflect()) {
                    return Some(false);
                }
            } else {
                return Some(false);
            }
        }
        EnumVariant::Struct(s_a) => {
            if let EnumVariant::Struct(s_b) = variant_b {
                if let Some(false) | None = s_b.reflect_partial_eq(s_a.as_reflect()) {
                    return Some(false);
                }
            } else {
                return Some(false);
            }
        }
    }
    Some(true)
}
