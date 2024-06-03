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
    /// Corresponds to [`Dir2::Y`] and [`Dir2::NORTH`]
    North,
    /// Corresponds to [`Dir2::X`] and [`Dir2::EAST`]
    East,
    /// Corresponds to [`Dir2::NEG_X`] and [`Dir2::SOUTH`]
    South,
    /// Corresponds to [`Dir2::NEG_Y`] and [`Dir2::WEST`]
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
    /// Corresponds to [`Dir2::Y`] and [`Dir2::NORTH`]
    North,
    /// Corresponds to [`Dir2::NORTH_EAST`]
    NorthEast,
    /// Corresponds to [`Dir2::X`] and [`Dir2::EAST`]
    East,
    /// Corresponds to [`Dir2::SOUTH_EAST`]
    SouthEast,
    /// Corresponds to [`Dir2::NEG_X`] and [`Dir2::SOUTH`]
    South,
    /// Corresponds to [`Dir2::SOUTH_WEST`]
    SouthWest,
    /// Corresponds to [`Dir2::NEG_Y`] and [`Dir2::WEST`]
    West,
    /// Corresponds to [`Dir2::NORTH_WEST`]
    NorthWest,
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

#[cfg(test)]
mod test_compass_quadrant {
    use crate::{CompassQuadrant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
}

#[cfg(test)]
mod test_compass_octant {
    use crate::{CompassOctant, Dir2, Vec2};

    #[test]
    fn test_cardinal_directions() {
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
        let tests = vec![
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
}
