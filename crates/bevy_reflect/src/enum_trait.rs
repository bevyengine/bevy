use crate::{Reflect, Struct, Tuple};

pub trait Enum: Reflect {
    fn variant(&self) -> EnumVariant<'_>;
    fn variant_mut(&mut self) -> EnumVariantMut<'_>;
    fn variant_info(&self) -> VariantInfo<'_>;
    fn iter_variants_info(&self) -> VariantInfoIter<'_>;
    fn get_index_name(&self, index: usize) -> Option<&str>;
    fn get_index_from_name(&self, name: &str) -> Option<usize>;
}
pub struct VariantInfo<'a> {
    pub index: usize,
    pub name: &'a str,
}
pub struct VariantInfoIter<'a> {
    pub(crate) value: &'a dyn Enum,
    pub(crate) index: usize,
    pub(crate) len: usize,
}
impl<'a> Iterator for VariantInfoIter<'a> {
    type Item = VariantInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }
        let item = VariantInfo {
            index: self.index,
            name: self.value.get_index_name(self.index).unwrap(),
        };
        self.index += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.len - self.index;
        (size, Some(size))
    }
}
impl<'a> ExactSizeIterator for VariantInfoIter<'a> {}

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
