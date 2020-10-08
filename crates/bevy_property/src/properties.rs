use crate::{DynamicProperties, Property, PropertyType, PropertyVal};

pub trait Properties: Property {
    fn prop(&self, name: &str) -> Option<&dyn Property>;
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Property>;
    fn prop_with_index(&self, index: usize) -> Option<&dyn Property>;
    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Property>;
    fn prop_name(&self, index: usize) -> Option<&str>;
    fn prop_len(&self) -> usize;
    fn iter_props(&self) -> PropertyIter;
    fn set_prop(&mut self, name: &str, value: &dyn Property) {
        if let Some(prop) = self.prop_mut(name) {
            prop.set(value);
        } else {
            panic!("prop does not exist: {}", name);
        }
    }
    fn to_dynamic(&self) -> DynamicProperties {
        let mut dynamic_props = match self.property_type() {
            PropertyType::Map => {
                let mut dynamic_props = DynamicProperties::map();
                for (i, prop) in self.iter_props().enumerate() {
                    let name = self
                        .prop_name(i)
                        .expect("All properties in maps should have a name");
                    dynamic_props.set_box(name, prop.clone_prop());
                }
                dynamic_props
            }
            PropertyType::Seq => {
                let mut dynamic_props = DynamicProperties::seq();
                for prop in self.iter_props() {
                    dynamic_props.push(prop.clone_prop(), None);
                }
                dynamic_props
            }
            _ => panic!("Properties cannot be Value types"),
        };

        dynamic_props.type_name = self.type_name().to_string();
        dynamic_props
    }
}

pub struct PropertyIter<'a> {
    pub(crate) props: &'a dyn Properties,
    pub(crate) index: usize,
}

impl<'a> PropertyIter<'a> {
    pub fn new(props: &'a dyn Properties) -> Self {
        PropertyIter { props, index: 0 }
    }
}

impl<'a> Iterator for PropertyIter<'a> {
    type Item = &'a dyn Property;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.props.prop_len() {
            let prop = self.props.prop_with_index(self.index).unwrap();
            self.index += 1;
            Some(prop)
        } else {
            None
        }
    }
}

pub trait PropertiesVal {
    fn prop_val<T: 'static>(&self, name: &str) -> Option<&T>;
    fn set_prop_val<T: 'static>(&mut self, name: &str, value: T);
}

impl<P> PropertiesVal for P
where
    P: Properties,
{
    #[inline]
    fn prop_val<T: 'static>(&self, name: &str) -> Option<&T> {
        self.prop(name).and_then(|p| p.any().downcast_ref::<T>())
    }

    #[inline]
    fn set_prop_val<T: 'static>(&mut self, name: &str, value: T) {
        if let Some(prop) = self.prop_mut(name) {
            prop.set_val(value);
        } else {
            panic!("prop does not exist or is incorrect type: {}", name);
        }
    }
}
