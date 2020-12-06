use bevy_core::Name;
use bevy_type_registry::TypeUuid;

use super::curve::CurveUntyped;
use super::hierarchy::Hierarchy;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

#[derive(Debug, Clone, Copy)]
pub struct CurveEntry {
    pub entity_index: u16,
    pub property_index: u16,
}

// TODO: impl Serialize, Deserialize
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification made by parent index and name
    hierarchy: Hierarchy,
    /// Each curve as one entry that maps a curve to an entity and a property path
    entries: Vec<CurveEntry>,
    curves: Vec<CurveUntyped>,
    /// A single property is made by string that combines
    /// component name followed by their attributes spaced by a period,
    /// like so: `"Transform.translation.x"`
    properties: Vec<Name>,
}

// fn clip_default_warp() -> bool {
//     true
// }

impl Default for Clip {
    fn default() -> Self {
        Self {
            warp: true,
            duration: 0.0,
            // ? NOTE: Since the root has no parent in this context it points to a place outside the vec bounds
            hierarchy: Hierarchy::default(),
            properties: vec![],
            curves: vec![],
            entries: vec![],
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    ///
    /// *NOTE* This is a expensive function
    pub fn add_animated_prop(&mut self, property_path: &str, mut curve: CurveUntyped) {
        // Clip an only have some amount of curves and entities
        // this limitation was added to save memory (but you can increase it if you want)
        assert!(
            (self.curves.len() as u16) <= u16::MAX,
            "curve limit reached"
        );

        // Split in entity and attribute path,
        // NOTE: use rfind because it's expected the latter to be generally shorter
        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        let (entity_index, just_created) = self.hierarchy.get_or_insert_entity(path.0);
        let target_name = path.1.split_at(1).1;

        // If some entity was created it means this property is a new one so we can safely skip the attribute testing
        if !just_created {
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.entity_index != entity_index {
                    continue;
                }

                let property_name = &self.properties[entry.property_index as usize];
                let mid = target_name.len().min(property_name.len());
                let (head0, tail0) = property_name.split_at(mid);
                let (head1, tail1) = target_name.split_at(mid);

                if head0 == head1 {
                    // Replace
                    if tail0.len() == 0 && tail1.len() == 0 {
                        // Found a property are equal the one been inserted
                        // Replace curve, the property was already added, this is very important
                        // because it guarantees that each property will have unique access to some
                        // attribute during the update stages

                        let inserted_duration = curve.duration();
                        std::mem::swap(&mut self.curves[i], &mut curve);
                        self.update_duration(curve.duration(), inserted_duration);
                        return;
                    } else {
                        // Check the inserted attribute is nested of an already present attribute
                        // NOTE: ".../Enity0@Transform.translation" and ".../Enity0@Transform.translation.x"
                        // can't coexist because it may cause a problems of non unique access
                        if tail0.starts_with('.') || tail1.starts_with('.') {
                            panic!("nesting properties");
                        }
                    }
                }
            }
        }

        // Find property or insert as a new one
        let target_name = Name::from_str(target_name);
        let property_index = self
            .properties
            .iter()
            .position(|property_name| property_name == &target_name)
            .map_or_else(
                || {
                    let property_index = self.properties.len() as u16;
                    self.properties.push(target_name);
                    property_index
                },
                |property_index| property_index as u16,
            );

        self.duration = self.duration.max(curve.duration());
        self.entries.push(CurveEntry {
            entity_index,
            property_index,
        });
        self.curves.push(curve);
    }

    /// Number of animated properties in this clip
    #[inline(always)]
    pub fn len(&self) -> u16 {
        self.curves.len() as u16
    }

    /// Returns the property curve property path.
    ///
    /// The clip stores a property path in a specific way to improve search performance
    /// thus it needs to rebuilt the curve property path in the human readable format
    pub fn get_property_path(&self, index: u16) -> String {
        let CurveEntry {
            entity_index,
            property_index,
        } = &self.entries[index as usize];

        format!(
            "{}@{}",
            self.hierarchy
                .get_entity_path_at(*entity_index)
                .expect("property as an invalid entity"),
            self.properties[*property_index as usize].as_str()
        )
    }

    /// Clip duration
    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Updates the clip duration based of the inserted and removed curve durations
    /// *WARNING* The curve must be already replaced before call this function
    fn update_duration(&mut self, removed_duration: f32, inserted_duration: f32) {
        if removed_duration == inserted_duration {
            // If precisely matches the inserted curve duration we don't need to update anything
        } else if float_cmp::approx_eq!(f32, removed_duration, self.duration, ulps = 2) {
            // At this point the duration for the removed curve is the same as self.duration
            // this mean that we have to compute the clip duration from scratch;
            //
            // NOTE: I opted for am approximated float comparison because it's better
            // to do all the this work than get a hard to debug glitch
            // TODO: Review approximated comparison

            self.duration = self
                .curves
                .iter()
                .map(|c| c.duration())
                .fold(0.0, |acc, x| acc.max(x));
        } else {
            // Faster clip duration update
            self.duration = self.duration.max(inserted_duration);
        }
    }

    #[inline(always)]
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hierarchy
    }

    #[inline(always)]
    pub fn properties(&self) -> &[Name] {
        &self.properties[..]
    }

    #[inline(always)]
    pub fn curves(&self) -> impl Iterator<Item = (&CurveEntry, &CurveUntyped)> {
        self.entries.iter().zip(self.curves.iter())
    }

    #[inline(always)]
    pub fn get(&self, curve_index: u16) -> Option<&CurveUntyped> {
        self.curves.get(curve_index as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use bevy_math::prelude::*;

    #[test]
    fn create_clip() {
        let mut clip = Clip::default();
        let curve = Curve::from_linear(0.0, 1.0, 0.0, 1.0);
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(prop, CurveUntyped::Float(curve));
        assert_eq!(clip.get_property_path(0), prop);
    }

    #[test]
    fn clip_replace_property() {
        // NOTE: This test is very important because it guarantees that each property have unique
        // access to some attribute during the update stages
        let mut clip = Clip::default();
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.y",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        assert_eq!(clip.duration(), 1.0);
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.1, 2.0)),
        );
        assert_eq!(clip.len(), 2);
        assert_eq!(clip.get_property_path(1), prop);
        assert_eq!(clip.duration(), 1.2);
    }

    #[test]
    fn clip_fine_grain_properties() {
        let mut clip = Clip::default();
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.y",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        assert_eq!(clip.duration(), 1.0);
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.x",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.0, 2.0)),
        );
        assert_eq!(clip.len(), 2);
        assert_eq!(clip.duration(), 1.2);
    }

    #[test]
    #[should_panic]
    fn clip_nested_properties() {
        // Maybe required by `bevy_reflect` this guarantees are necessary
        // because the way we execute
        let mut clip = Clip::default();
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate",
            CurveUntyped::Vec3(Curve::from_linear(0.0, 1.0, Vec3::zero(), Vec3::unit_y())),
        );
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.x",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.0, 2.0)),
        );
    }
}
