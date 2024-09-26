use crate::type_info::impl_type_methods;
use crate::{Type, TypePath};
use alloc::borrow::Cow;
use bevy_utils::HashMap;
use core::fmt::{Debug, Formatter};

#[derive(Clone, Default)]
pub struct Generics {
    infos: Box<[GenericInfo]>,
    index_map: HashMap<Cow<'static, str>, usize>,
}

impl Debug for Generics {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();
        list.entries(self.infos.iter());
        list.finish()
    }
}

impl Generics {
    pub fn new() -> Self {
        Self {
            infos: Box::new([]),
            index_map: HashMap::new(),
        }
    }

    pub fn with<T: TypePath + ?Sized>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        is_const: bool,
    ) -> Self {
        let name = name.into();
        self.index_map.insert(name.clone(), self.infos.len());
        self.infos = IntoIterator::into_iter(self.infos)
            .chain(core::iter::once(GenericInfo::new::<T>(name, is_const)))
            .collect();
        self
    }
}

impl FromIterator<GenericInfo> for Generics {
    fn from_iter<T: IntoIterator<Item = GenericInfo>>(iter: T) -> Self {
        let mut index_map = HashMap::new();
        let infos = iter
            .into_iter()
            .enumerate()
            .map(|(index, info)| {
                index_map.insert(info.name().clone(), index);
                info
            })
            .collect::<Vec<_>>()
            .into();

        Self { infos, index_map }
    }
}

#[derive(Clone, Debug)]
pub struct GenericInfo {
    name: Cow<'static, str>,
    ty: Type,
    is_const: bool,
}

impl GenericInfo {
    pub fn new<T: TypePath + ?Sized>(name: impl Into<Cow<'static, str>>, is_const: bool) -> Self {
        Self {
            name: name.into(),
            ty: Type::of::<T>(),
            is_const,
        }
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn is_const(&self) -> bool {
        self.is_const
    }

    impl_type_methods!(ty);
}

macro_rules! impl_generic_info_methods {
    ($field:ident) => {
        pub fn with_generics(mut self, generics: crate::generics::Generics) -> Self {
            self.$field = generics;
            self
        }

        pub fn generics(&self) -> &crate::generics::Generics {
            &self.$field
        }
    };
}

pub(crate) use impl_generic_info_methods;
