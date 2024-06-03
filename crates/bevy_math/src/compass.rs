use crate::Dir2;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A compass enum with 4 directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub enum CompassQuadrant {
    /// The north direction.
    North,
    /// The east direction.
    East,
    /// The south direction.
    South,
    /// The west direction.
    West,
}

/// A compass enum with 8 directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub enum CompassOctant {
    /// The north direction.
    North,
    /// The north east direction.
    NorthEast,
    /// The east direction.
    East,
    /// The south east direction.
    SouthEast,
    /// The south direction.
    South,
    /// The south west direction.
    SouthWest,
    /// The west direction.
    West,
    /// The north west direction.
    NorthWest,
}

impl From<CompassQuadrant> for Dir2 {
    /// [`CompassQuadrant::North`] corresponds to [`Dir2::Y`].
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
    /// [`CompassQuadrant::North`] corresponds to [`Dir2::Y`].
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
    /// [`CompassQuadrant::North`] corresponds to [`Dir2::Y`].
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
    /// [`CompassQuadrant::North`] corresponds to [`Dir2::Y`].
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

#[cfg(test)]
mod test_compass_quadrant {
    use crate::{CompassQuadrant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(1.0, 0.0).normalize()),
                CompassQuadrant::East,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.0, 1.0).normalize()),
                CompassQuadrant::North,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-1.0, 0.0).normalize()),
                CompassQuadrant::West,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.0, -1.0).normalize()),
                CompassQuadrant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.1, 0.9).normalize()),
                CompassQuadrant::North,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.1, 0.9).normalize()),
                CompassQuadrant::North,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_east_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(0.9, 0.1).normalize()),
                CompassQuadrant::East,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.9, -0.1).normalize()),
                CompassQuadrant::East,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.1, -0.9).normalize()),
                CompassQuadrant::South,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.1, -0.9).normalize()),
                CompassQuadrant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }

    #[test]
    fn test_west_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.9, -0.1).normalize()),
                CompassQuadrant::West,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-0.9, 0.1).normalize()),
                CompassQuadrant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassQuadrant::from(dir), expected);
        }
    }
}

#[cfg(test)]
mod test_compass_octant {
    use crate::{CompassOctant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.5, 0.5).normalize()),
                CompassOctant::NorthWest,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.0, 1.0).normalize()),
                CompassOctant::North,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.5, 0.5).normalize()),
                CompassOctant::NorthEast,
            ),
            (
                Dir2::new_unchecked(Vec2::new(1.0, 0.0).normalize()),
                CompassOctant::East,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.5, -0.5).normalize()),
                CompassOctant::SouthEast,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.0, -1.0).normalize()),
                CompassOctant::South,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-0.5, -0.5).normalize()),
                CompassOctant::SouthWest,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-1.0, 0.0).normalize()),
                CompassOctant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.1, 0.9).normalize()),
                CompassOctant::North,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.1, 0.9).normalize()),
                CompassOctant::North,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_east_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(0.4, 0.6).normalize()),
                CompassOctant::NorthEast,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.6, 0.4).normalize()),
                CompassOctant::NorthEast,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_east_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(0.9, 0.1).normalize()),
                CompassOctant::East,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.9, -0.1).normalize()),
                CompassOctant::East,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_east_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(0.4, -0.6).normalize()),
                CompassOctant::SouthEast,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.6, -0.4).normalize()),
                CompassOctant::SouthEast,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.1, -0.9).normalize()),
                CompassOctant::South,
            ),
            (
                Dir2::new_unchecked(Vec2::new(0.1, -0.9).normalize()),
                CompassOctant::South,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_south_west_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.4, -0.6).normalize()),
                CompassOctant::SouthWest,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-0.6, -0.4).normalize()),
                CompassOctant::SouthWest,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_west_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.9, -0.1).normalize()),
                CompassOctant::West,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-0.9, 0.1).normalize()),
                CompassOctant::West,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }

    #[test]
    fn test_north_west_pie_slice() {
        let tests = vec![
            (
                Dir2::new_unchecked(Vec2::new(-0.4, 0.6).normalize()),
                CompassOctant::NorthWest,
            ),
            (
                Dir2::new_unchecked(Vec2::new(-0.6, 0.4).normalize()),
                CompassOctant::NorthWest,
            ),
        ];

        for (dir, expected) in tests {
            assert_eq!(CompassOctant::from(dir), expected);
        }
    }
}
