use bevy_type_registry::TypeUuid;

use super::curve::CurveUntyped;
use super::hierarchy::Hierarchy;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// TODO: impl Serialize, Deserialize
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification made by parent index and name
    pub(crate) hierarchy: Hierarchy,
    /// Attribute is made by the entity index and a string that combines
    /// component name followed by their attributes spaced by a period,
    /// like so: `"Transform.translation.x"`
    pub(crate) properties: Vec<(u16, String)>,
    pub(crate) curves: Vec<CurveUntyped>,
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

        let (entity1, just_created) = self.hierarchy.get_or_insert_entity(path.0);
        let name1 = path.1.split_at(1).1;

        // If some entity was created it means this property is a new one so we can safely skip the attribute testing
        if !just_created {
            for (i, entry) in self.properties.iter().enumerate() {
                let (entity0, name0) = entry;

                if *entity0 != entity1 {
                    continue;
                }

                let mid = name1.len().min(name0.len());
                let (head0, tail0) = name0.split_at(mid);
                let (head1, tail1) = name1.split_at(mid);

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

        self.duration = self.duration.max(curve.duration());
        self.properties.push((entity1, name1.to_string()));
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
        let (entity_index, name) = &self.properties[index as usize];
        format!(
            "{}@{}",
            self.hierarchy
                .get_entity_path_at(*entity_index)
                .expect("property as an invalid entity"),
            name.as_str()
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
}
