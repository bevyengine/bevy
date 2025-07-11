use crate::world::World;

use super::{run_this_packet_system, Packet, SystemInput};

pub trait OptionPacket {
    fn run(self, world: &mut World);
}
impl OptionPacket for (){ fn run(self, _: &mut World) {} }

impl<E: Packet> OptionPacket for E
where 
    for<'e> E: SystemInput<Inner<'e> = E>,
{
    fn run(self, world: &mut World) {
        run_this_packet_system::<E>(self, world);
    }
}
impl<O: OptionPacket> OptionPacket for Option<O> {
    fn run(self, world: &mut World) {
        let Some(event) = self else {return};
        event.run(world);
    }
}
macro_rules! impl_option_event_tuple {
    ($($param: ident),*) => {
        impl<$($param: OptionPacket,)*> OptionPacket for ($($param,)*) {
            fn run(self, world: &mut World) {
                #[allow(non_snake_case)]
                let ($($param,)*) = self;
                $(
                    $param.run(world);
                )*
            }
        }
    }
}

impl_option_event_tuple!(O1);
impl_option_event_tuple!(O1, O2);
impl_option_event_tuple!(O1, O2, O3);
impl_option_event_tuple!(O1, O2, O3, O4);
impl_option_event_tuple!(O1, O2, O3, O4, O5);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11, O12);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11, O12, O13);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11, O12, O13, O14);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11, O12, O13, O14, O15);
impl_option_event_tuple!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10, O11, O12, O13, O14, O15, O16);

macro_rules! impl_option_packet_array {
    ($($N: literal),*) => {
        $(
            impl<O: OptionPacket> OptionPacket for [O;$N] {
                fn run(self, world: &mut World) {
                    for packet in self {
                        packet.run(world);
                    }
                }
            }
        )*
    };
}

impl_option_packet_array!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
