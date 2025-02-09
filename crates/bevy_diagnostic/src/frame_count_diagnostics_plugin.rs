use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

#[cfg(feature = "serialize")]
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

/// Maintains a count of frames rendered since the start of the application.
///
/// [`FrameCount`] is incremented during [`Last`], providing predictable
/// behavior: it will be 0 during the first update, 1 during the next, and so forth.
///
/// # Overflows
///
/// [`FrameCount`] will wrap to 0 after exceeding [`u32::MAX`]. Within reasonable
/// assumptions, one may exploit wrapping arithmetic to determine the number of frames
/// that have elapsed between two observations – see [`u32::wrapping_sub()`].
#[derive(Debug, Default, Resource, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameCount(pub u32);

/// Adds frame counting functionality to Apps.
#[derive(Default)]
pub struct FrameCountPlugin;

impl Plugin for FrameCountPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>();
        app.add_systems(Last, update_frame_count);
    }
}

/// A system used to increment [`FrameCount`] with wrapping addition.
///
/// See [`FrameCount`] for more details.
pub fn update_frame_count(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 = frame_count.0.wrapping_add(1);
}

#[cfg(feature = "serialize")]
// Manually implementing serialize/deserialize allows us to use a more compact representation as simple integers
impl Serialize for FrameCount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

#[cfg(feature = "serialize")]
impl<'de> Deserialize<'de> for FrameCount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_u32(FrameVisitor)
    }
}

#[cfg(feature = "serialize")]
struct FrameVisitor;

#[cfg(feature = "serialize")]
impl<'de> Visitor<'de> for FrameVisitor {
    type Value = FrameCount;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str(core::any::type_name::<FrameCount>())
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FrameCount(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counter_update() {
        let mut app = App::new();
        app.add_plugins(FrameCountPlugin);
        app.update();

        let frame_count = app.world().resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}

#[cfg(all(test, feature = "serialize"))]
mod serde_tests {
    use super::*;

    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_serde_frame_count() {
        let frame_count = FrameCount(100);
        assert_tokens(&frame_count, &[Token::U32(100)]);
    }
}
