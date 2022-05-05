use super::{In, ParamSet, SystemParam, SystemParamFunction, SystemParamItem};

/// A [`System`](crate::system::System) that chains two systems together, creating a new system that routes the output of
/// the first system into the input of the second system, yielding the output of the second system.
///
/// Given two systems A and B, A may be chained with B as `chain(A, B)` if the return type of A is
/// equal to the input type of B.
///
/// Note that the input to a system is the [`In`] parameter, which must be the first parameter to the
/// system function. If the function has no designated input parameter, the input type is [`()`](unit)
///
/// # Examples
///
/// ```
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut world = World::default();
///     world.insert_resource(Message("42".to_string()));
///
///     // chain the `parse_message_system`'s output into the `filter_system`s input
///     let mut chained_system = IntoSystem::into_system(chain(parse_message_system, filter_system));
///     chained_system.initialize(&mut world);
///     assert_eq!(chained_system.run((), &mut world), Some(42));
/// }
///
/// struct Message(String);
///
/// fn parse_message_system(message: Res<Message>) -> Result<usize, ParseIntError> {
///     message.0.parse::<usize>()
/// }
///
/// fn filter_system(In(result): In<Result<usize, ParseIntError>>) -> Option<usize> {
///     result.ok().filter(|&n| n < 100)
/// }
/// ```
///
/// [`In`]: crate::system::In
pub fn chain<AIn, Shared, BOut, A, AParam, AMarker, B, BParam, BMarker>(
    mut a: A,
    mut b: B,
) -> impl FnMut(In<AIn>, ParamSet<(SystemParamItem<AParam>, SystemParamItem<BParam>)>) -> BOut
where
    A: SystemParamFunction<AIn, Shared, AParam, AMarker>,
    B: SystemParamFunction<Shared, BOut, BParam, BMarker>,
    AParam: SystemParam,
    BParam: SystemParam,
{
    move |In(a_in), mut params| {
        let shared = a.run(a_in, params.p0());
        b.run(shared, params.p1())
    }
}
