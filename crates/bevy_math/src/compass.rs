use crate::Dir2;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use core::ops::Neg;
use glam::Vec2;

/// A compass enum with 4 directions.
/// ```text
///          N (North)
///          ▲
///          │
///          │
/// W (West) ┼─────► E (East)
///          │
///          │
///          ▼
///          S (South)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub enum CompassQuadrant {
    /// Corresponds to [`Dir2::Y`] and [`Dir2::NORTH`]
    North,
    /// Corresponds to [`Dir2::X`] and [`Dir2::EAST`]
    East,
    /// Corresponds to [`Dir2::NEG_Y`] and [`Dir2::SOUTH`]
    South,
    /// Corresponds to [`Dir2::NEG_X`] and [`Dir2::WEST`]
    West,
}

impl CompassQuadrant {
    /// Converts a standard index to a [`CompassQuadrant`].
    ///
    /// Starts at 0 for [`CompassQuadrant::North`] and increments clockwise.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::North),
            1 => Some(Self::East),
            2 => Some(Self::South),
            3 => Some(Self::West),
            _ => None,
        }
    }

    /// Converts a [`CompassQuadrant`] to a standard index.
    ///
    /// Starts at 0 for [`CompassQuadrant::North`] and increments clockwise.
    pub const fn to_index(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    /// Returns the opposite [`CompassQuadrant`], located 180 degrees from `self`.
    ///
    /// This can also be accessed via the `-` operator, using the [`Neg`] trait.
    pub const fn opposite(&self) -> CompassQuadrant {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    /// Checks if a point is in the direction represented by this [`CompassQuadrant`] from an origin.
    ///
    /// This uses a cone-based check: the vector from origin to the candidate point
    /// must have a positive dot product with the direction vector.
    ///
    /// Uses standard mathematical coordinates where Y increases upward.
    ///
    /// # Arguments
    ///
    /// * `origin` - The starting position
    /// * `candidate` - The target position to check
    ///
    /// # Returns
    ///
    /// `true` if the candidate is generally in the direction of this quadrant from the origin.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_math::{CompassQuadrant, Vec2};
    ///
    /// let origin = Vec2::new(0.0, 0.0);
    /// let north_point = Vec2::new(0.0, 10.0);  // Above origin (Y+ = up)
    /// let east_point = Vec2::new(10.0, 0.0);   // Right of origin
    ///
    /// assert!(CompassQuadrant::North.is_in_direction(origin, north_point));
    /// assert!(!CompassQuadrant::North.is_in_direction(origin, east_point));
    /// ```
    pub fn is_in_direction(self, origin: Vec2, candidate: Vec2) -> bool {
        let dir = Dir2::from(self);
        let to_candidate = candidate - origin;
        to_candidate.dot(*dir) > 0.0
    }
}

/// A compass enum with 8 directions.
/// ```text
///          N (North)
///          ▲
///     NW   │   NE
///        ╲ │ ╱
/// W (West) ┼─────► E (East)
///        ╱ │ ╲
///     SW   │   SE
///          ▼
///          S (South)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub enum CompassOctant {
    /// Corresponds to [`Dir2::Y`] and [`Dir2::NORTH`]
    North,
    /// Corresponds to [`Dir2::NORTH_EAST`]
    NorthEast,
    /// Corresponds to [`Dir2::X`] and [`Dir2::EAST`]
    East,
    /// Corresponds to [`Dir2::SOUTH_EAST`]
    SouthEast,
    /// Corresponds to [`Dir2::NEG_Y`] and [`Dir2::SOUTH`]
    South,
    /// Corresponds to [`Dir2::SOUTH_WEST`]
    SouthWest,
    /// Corresponds to [`Dir2::NEG_X`] and [`Dir2::WEST`]
    West,
    /// Corresponds to [`Dir2::NORTH_WEST`]
    NorthWest,
}

impl CompassOctant {
    /// Converts a standard index to a [`CompassOctant`].
    ///
    /// Starts at 0 for [`CompassOctant::North`] and increments clockwise.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::North),
            1 => Some(Self::NorthEast),
            2 => Some(Self::East),
            3 => Some(Self::SouthEast),
            4 => Some(Self::South),
            5 => Some(Self::SouthWest),
            6 => Some(Self::West),
            7 => Some(Self::NorthWest),
            _ => None,
        }
    }

    /// Converts a [`CompassOctant`] to a standard index.
    ///
    /// Starts at 0 for [`CompassOctant::North`] and increments clockwise.
    pub const fn to_index(self) -> usize {
        match self {
            Self::North => 0,
            Self::NorthEast => 1,
            Self::East => 2,
            Self::SouthEast => 3,
            Self::South => 4,
            Self::SouthWest => 5,
            Self::West => 6,
            Self::NorthWest => 7,
        }
    }

    /// Returns the opposite [`CompassOctant`], located 180 degrees from `self`.
    ///
    /// This can also be accessed via the `-` operator, using the [`Neg`] trait.
    pub const fn opposite(&self) -> CompassOctant {
        match self {
            Self::North => Self::South,
            Self::NorthEast => Self::SouthWest,
            Self::East => Self::West,
            Self::SouthEast => Self::NorthWest,
            Self::South => Self::North,
            Self::SouthWest => Self::NorthEast,
            Self::West => Self::East,
            Self::NorthWest => Self::SouthEast,
        }
    }

    /// Checks if a point is in the direction represented by this [`CompassOctant`] from an origin.
    ///
    /// This uses a cone-based check: the vector from origin to the candidate point
    /// must have a positive dot product with the direction vector.
    ///
    /// Uses standard mathematical coordinates where Y increases upward.
    ///
    /// # Arguments
    ///
    /// * `origin` - The starting position
    /// * `candidate` - The target position to check
    ///
    /// # Returns
    ///
    /// `true` if the candidate is generally in the direction of this octant from the origin.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_math::{CompassOctant, Vec2};
    ///
    /// let origin = Vec2::new(0.0, 0.0);
    /// let north_point = Vec2::new(0.0, 10.0);  // Above origin (Y+ = up)
    /// let east_point = Vec2::new(10.0, 0.0);   // Right of origin
    ///
    /// assert!(CompassOctant::North.is_in_direction(origin, north_point));
    /// assert!(!CompassOctant::North.is_in_direction(origin, east_point));
    /// ```
    pub fn is_in_direction(self, origin: Vec2, candidate: Vec2) -> bool {
        let dir = Dir2::from(self);
        let to_candidate = candidate - origin;
        to_candidate.dot(*dir) > 0.0
    }
}

impl From<CompassQuadrant> for Dir2 {
    fn from(q: CompassQuadrant) -> Self {
        match q {
            CompassQuadrant::North => Dir2::NORTH,
            CompassQuadrant::East => Dir2::EAST,
            CompassQuadrant::South => Dir2::SOUTH,
            CompassQuadrant::West => Dir2::WEST,
        }
    }
}

impl From<Dir2> for CompassQuadrant {
    /// Converts a [`Dir2`] to a [`CompassQuadrant`] in a lossy manner.
    /// Converting back to a [`Dir2`] is not guaranteed to yield the same value.
    fn from(dir: Dir2) -> Self {
        let angle = dir.to_angle().to_degrees();

        match angle {
            -135.0..=-45.0 => Self::South,
            -45.0..=45.0 => Self::East,
            45.0..=135.0 => Self::North,
            135.0..=180.0 | -180.0..=-135.0 => Self::West,
            _ => unreachable!(),
        }
    }
}

impl From<CompassOctant> for Dir2 {
    fn from(o: CompassOctant) -> Self {
        match o {
            CompassOctant::North => Dir2::NORTH,
            CompassOctant::NorthEast => Dir2::NORTH_EAST,
            CompassOctant::East => Dir2::EAST,
            CompassOctant::SouthEast => Dir2::SOUTH_EAST,
            CompassOctant::South => Dir2::SOUTH,
            CompassOctant::SouthWest => Dir2::SOUTH_WEST,
            CompassOctant::West => Dir2::WEST,
            CompassOctant::NorthWest => Dir2::NORTH_WEST,
        }
    }
}

impl From<Dir2> for CompassOctant {
    /// Converts a [`Dir2`] to a [`CompassOctant`] in a lossy manner.
    /// Converting back to a [`Dir2`] is not guaranteed to yield the same value.
    fn from(dir: Dir2) -> Self {
        let angle = dir.to_angle().to_degrees();

        match angle {
            -112.5..=-67.5 => Self::South,
            -67.5..=-22.5 => Self::SouthEast,
            -22.5..=22.5 => Self::East,
            22.5..=67.5 => Self::NorthEast,
            67.5..=112.5 => Self::North,
            112.5..=157.5 => Self::NorthWest,
            157.5..=180.0 | -180.0..=-157.5 => Self::West,
            -157.5..=-112.5 => Self::SouthWest,
            _ => unreachable!(),
        }
    }
}

impl Neg for CompassQuadrant {
    type Output = CompassQuadrant;

    fn neg(self) -> Self::Output {
        self.opposite()
    }
}

impl Neg for CompassOctant {
    type Output = CompassOctant;

    fn neg(self) -> Self::Output {
        self.opposite()
    }
}

#[cfg(test)]
mod test_compass_quadrant {
    use crate::{CompassQuadrant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = [
            (
                Dir2::new(Vec2::new(1.0, 0.0)).unwrap(),
                CompassQuadrant::East,
            ),
            (
                Dir2::new(Vec2::new(0.0, 1.0)).unwrap(),
                CompassQuadrant::North,
            ),
            (
                Dir2::new(Vec2::new(-1.0, 0.0)).unwrap(),
                CompassQuadrant::West,
            ),
            (
                Dir2::new(Vec2::new(0.0, -1.0)).unwrap(),
                CompassQuadrant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.1, 0.9)).unwrap(),
                CompassQuadrant::North,
            ),
            (
                Dir2::new(Vec2::new(0.1, 0.9)).unwrap(),
                CompassQuadrant::North,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_east_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(0.9, 0.1)).unwrap(),
                CompassQuadrant::East,
            ),
            (
                Dir2::new(Vec2::new(0.9, -0.1)).unwrap(),
                CompassQuadrant::East,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.1, -0.9)).unwrap(),
                CompassQuadrant::South,
            ),
            (
                Dir2::new(Vec2::new(0.1, -0.9)).unwrap(),
                CompassQuadrant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_west_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.9, -0.1)).unwrap(),
                CompassQuadrant::West,
            ),
            (
                Dir2::new(Vec2::new(-0.9, 0.1)).unwrap(),
                CompassQuadrant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn out_of_bounds_indexes_return_none() {
        assert_eq!(CompassQuadrant::from_index(4), None);
        assert_eq!(CompassQuadrant::from_index(5), None);
        assert_eq!(CompassQuadrant::from_index(usize::MAX), None);
    }

    #[test]
    fn compass_indexes_are_reversible() {
        for i in 0..4 {
            let quadrant = CompassQuadrant::from_index(i).unwrap();
            assert_eq!(quadrant.to_index(), i);
        }
    }

    #[test]
    fn opposite_directions_reverse_themselves() {
        for i in 0..4 {
            let quadrant = CompassQuadrant::from_index(i).unwrap();
            assert_eq!(-(-quadrant), quadrant);
        }
    }
}

#[cfg(test)]
mod test_compass_octant {
    use crate::{CompassOctant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.5, 0.5)).unwrap(),
                CompassOctant::NorthWest,
            ),
            (
                Dir2::new(Vec2::new(0.0, 1.0)).unwrap(),
                CompassOctant::North,
            ),
            (
                Dir2::new(Vec2::new(0.5, 0.5)).unwrap(),
                CompassOctant::NorthEast,
            ),
            (Dir2::new(Vec2::new(1.0, 0.0)).unwrap(), CompassOctant::East),
            (
                Dir2::new(Vec2::new(0.5, -0.5)).unwrap(),
                CompassOctant::SouthEast,
            ),
            (
                Dir2::new(Vec2::new(0.0, -1.0)).unwrap(),
                CompassOctant::South,
            ),
            (
                Dir2::new(Vec2::new(-0.5, -0.5)).unwrap(),
                CompassOctant::SouthWest,
            ),
            (
                Dir2::new(Vec2::new(-1.0, 0.0)).unwrap(),
                CompassOctant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.1, 0.9)).unwrap(),
                CompassOctant::North,
            ),
            (
                Dir2::new(Vec2::new(0.1, 0.9)).unwrap(),
                CompassOctant::North,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_east_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(0.4, 0.6)).unwrap(),
                CompassOctant::NorthEast,
            ),
            (
                Dir2::new(Vec2::new(0.6, 0.4)).unwrap(),
                CompassOctant::NorthEast,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_east_pie_slice() {
        let tests = [
            (Dir2::new(Vec2::new(0.9, 0.1)).unwrap(), CompassOctant::East),
            (
                Dir2::new(Vec2::new(0.9, -0.1)).unwrap(),
                CompassOctant::East,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_east_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(0.4, -0.6)).unwrap(),
                CompassOctant::SouthEast,
            ),
            (
                Dir2::new(Vec2::new(0.6, -0.4)).unwrap(),
                CompassOctant::SouthEast,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.1, -0.9)).unwrap(),
                CompassOctant::South,
            ),
            (
                Dir2::new(Vec2::new(0.1, -0.9)).unwrap(),
                CompassOctant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_west_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.4, -0.6)).unwrap(),
                CompassOctant::SouthWest,
            ),
            (
                Dir2::new(Vec2::new(-0.6, -0.4)).unwrap(),
                CompassOctant::SouthWest,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_west_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.9, -0.1)).unwrap(),
                CompassOctant::West,
            ),
            (
                Dir2::new(Vec2::new(-0.9, 0.1)).unwrap(),
                CompassOctant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_west_pie_slice() {
        let tests = [
            (
                Dir2::new(Vec2::new(-0.4, 0.6)).unwrap(),
                CompassOctant::NorthWest,
            ),
            (
                Dir2::new(Vec2::new(-0.6, 0.4)).unwrap(),
                CompassOctant::NorthWest,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn out_of_bounds_indexes_return_none() {
        assert_eq!(CompassOctant::from_index(8), None);
        assert_eq!(CompassOctant::from_index(9), None);
        assert_eq!(CompassOctant::from_index(usize::MAX), None);
    }

    #[test]
    fn compass_indexes_are_reversible() {
        for i in 0..8 {
            let octant = CompassOctant::from_index(i).unwrap();
            assert_eq!(octant.to_index(), i);
        }
    }

    #[test]
    fn opposite_directions_reverse_themselves() {
        for i in 0..8 {
            let octant = CompassOctant::from_index(i).unwrap();
            assert_eq!(-(-octant), octant);
        }
    }
}
